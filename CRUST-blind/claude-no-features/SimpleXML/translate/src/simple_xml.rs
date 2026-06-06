use crate::simple_vector::Vector;

const BEGIN_TAG_TOKEN: char = '<';
const END_TAG_TOKEN: char = '>';
const SPLASH_TOKEN: char = '/';

pub struct XMLElement {
    pub tag_name: String,
    pub value: String,
    pub parent: (),
    pub children: Vector<XMLElement>, // owns children
}

#[derive(Debug, Clone, PartialEq)]
pub enum XMLTokenType {
    BeginOpenTag,  // “<”
    BeginCloseTag, // “</”
    EndTag,        // “>”
    Text,          // non‑empty, trimmed text
}

pub struct XMLToken {
    pub token_type: XMLTokenType,
    pub data: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParseState {
    State1,
    State2,
    State3,
    State4,
    State5,
    State6,
    State7,
    State8,
    StateError,
}

pub struct StackElement {
    element: XMLElement,
    depth: usize,
}

impl StackElement {
    pub fn new(element: XMLElement, depth: usize) -> Self {
        StackElement { element, depth }
    }
    pub fn release(&mut self) {
        // No explicit cleanup required in safe Rust; values are dropped automatically.
    }
}

impl XMLElement {
    pub fn new(tag_name: String, value: String) -> XMLElement {
        XMLElement {
            tag_name,
            value,
            parent: (),
            children: Vector::new(8),
        }
    }
}

pub struct XMLParser {
    input: String,
    position: usize,
    depth: usize,
    state: ParseState,

    tag_stack: Vector<String>,
    value_stack: Vector<String>,
    element_stack: Vector<StackElement>,
}

impl XMLParser {
    pub fn new() -> Self {
        XMLParser {
            input: String::new(),
            position: 0,
            depth: 0,
            state: ParseState::State1,
            tag_stack: Vector::new(8),
            value_stack: Vector::new(8),
            element_stack: Vector::new(8),
        }
    }

    pub fn parse(&mut self, text: &str) -> Result<XMLElement, String> {
        self.input = text.to_string();
        self.position = 0;
        self.depth = 0;
        self.state = ParseState::State1;
        self.tag_stack = Vector::new(8);
        self.value_stack = Vector::new(8);
        self.element_stack = Vector::new(8);

        loop {
            let token = match self.get_next_token() {
                Some(t) => t,
                None => break,
            };

            // Skip empty text tokens
            if token.token_type == XMLTokenType::Text && token.data.is_none() {
                continue;
            }

            let next_state = Self::translate(self.state, &token.token_type);

            if next_state != ParseState::StateError {
                match self.state {
                    ParseState::State1 => {}
                    ParseState::State2 => {
                        if token.token_type == XMLTokenType::Text {
                            if let Some(data) = token.data.clone() {
                                self.tag_stack.push_back(data);
                                self.depth += 1;
                            }
                        }
                    }
                    ParseState::State3 => {}
                    ParseState::State4 => {
                        // push value (may be empty if text was missing, but only Text token leads to State5)
                        let data = token.data.clone().unwrap_or_default();
                        self.value_stack.push_back(data);
                    }
                    ParseState::State5 => {}
                    ParseState::State6 => {
                        if token.token_type == XMLTokenType::Text {
                            // Validate closing tag matches the latest open tag.
                            let token_data = token.data.clone().unwrap_or_default();
                            let top_tag = self
                                .tag_stack
                                .top_back()
                                .cloned()
                                .unwrap_or_default();
                            if token_data != top_tag {
                                return Err(format!(
                                    "Mismatched closing tag: expected '{}', found '{}'",
                                    top_tag, token_data
                                ));
                            }
                        }
                    }
                    ParseState::State7 => {
                        if token.token_type == XMLTokenType::EndTag {
                            // Pop the current tag and value
                            let current_tag = self.tag_stack.top_back().cloned().unwrap_or_default();
                            let current_value =
                                self.value_stack.top_back().cloned().unwrap_or_default();
                            let length = self.element_stack.size();
                            if self.depth > 0 {
                                self.depth -= 1;
                            }

                            let mut current = XMLElement::new(current_tag, current_value);
                            let se_depth = self.depth;

                            // Find children of current elem: pop StackElements with depth > se.depth
                            // We need to push them to current.children using push_front order
                            // (matches C: pops from back of element_stack and pushes to front of children)
                            let mut popped: Vec<StackElement> = Vec::new();
                            for _ in 0..length {
                                let should_pop = match self.element_stack.top_back() {
                                    Some(elem) => elem.depth > se_depth,
                                    None => false,
                                };
                                if !should_pop {
                                    break;
                                }
                                if let Some(se) = self.element_stack.pop_back() {
                                    popped.push(se);
                                } else {
                                    break;
                                }
                            }
                            // C does push_front for each popped; iterating popped in pop order
                            // and pushing to front yields children in original (oldest-first) order.
                            for se in popped {
                                current.children.push_front(se.element);
                            }

                            // push to stack
                            let se = StackElement::new(current, se_depth);
                            self.element_stack.push_back(se);

                            self.tag_stack.pop_back();
                            self.value_stack.pop_back();
                        }
                    }
                    ParseState::State8 => {}
                    ParseState::StateError => {}
                }
            } else {
                return Err(format!(
                    "Parse error at position {} in state {:?}",
                    self.position, self.state
                ));
            }

            self.state = next_state;
        }

        // Pop the root element from the element_stack
        let root = self.element_stack.pop_back();
        self.release();
        match root {
            Some(se) => Ok(se.element),
            None => Err("No root element parsed".to_string()),
        }
    }

    fn get_next_token(&mut self) -> Option<XMLToken> {
        let bytes = self.input.as_bytes();
        let length = bytes.len();
        let begin_pos = self.position;

        if begin_pos >= length {
            return None;
        }

        while self.position < length {
            let ch = bytes[self.position] as char;
            self.position += 1;

            match ch {
                BEGIN_TAG_TOKEN => {
                    if self.position > begin_pos + 1 {
                        // We've encountered text before this tag; back up and emit text token
                        self.position -= 1;
                        let from = begin_pos;
                        let to = self.position - 1;
                        return Some(self.get_text_token(from, to));
                    } else {
                        // First char: check next
                        if self.position < length {
                            let next_char = bytes[self.position] as char;
                            if next_char == SPLASH_TOKEN {
                                self.position += 1;
                                return Some(XMLToken {
                                    token_type: XMLTokenType::BeginCloseTag,
                                    data: None,
                                });
                            } else {
                                return Some(XMLToken {
                                    token_type: XMLTokenType::BeginOpenTag,
                                    data: None,
                                });
                            }
                        } else {
                            return Some(XMLToken {
                                token_type: XMLTokenType::BeginOpenTag,
                                data: None,
                            });
                        }
                    }
                }
                END_TAG_TOKEN => {
                    if self.position > begin_pos + 1 {
                        self.position -= 1;
                        let from = begin_pos;
                        let to = self.position - 1;
                        return Some(self.get_text_token(from, to));
                    } else {
                        return Some(XMLToken {
                            token_type: XMLTokenType::EndTag,
                            data: None,
                        });
                    }
                }
                _ => {}
            }
        }

        // Reached end of input; return remaining text token
        Some(self.get_text_token(begin_pos, self.position - 1))
    }

    fn get_text_token(&self, mut from: usize, mut to: usize) -> XMLToken {
        let bytes = self.input.as_bytes();

        // trim leading spaces
        while from <= to && from < bytes.len() && bytes[from] == b' ' {
            from += 1;
        }
        // trim trailing spaces
        while to >= from && to < bytes.len() && bytes[to] == b' ' {
            if to == 0 {
                break;
            }
            to -= 1;
        }

        if to >= from && from < bytes.len() && to < bytes.len() {
            let s = std::str::from_utf8(&bytes[from..=to])
                .unwrap_or("")
                .to_string();
            XMLToken {
                token_type: XMLTokenType::Text,
                data: Some(s),
            }
        } else {
            XMLToken {
                token_type: XMLTokenType::Text,
                data: None,
            }
        }
    }

    fn translate(state: ParseState, token: &XMLTokenType) -> ParseState {
        // Mapping per the C state_translate table.
        match (state, token) {
            (ParseState::State1, XMLTokenType::BeginOpenTag) => ParseState::State2,
            (ParseState::State2, XMLTokenType::Text) => ParseState::State3,
            (ParseState::State3, XMLTokenType::EndTag) => ParseState::State4,
            (ParseState::State4, XMLTokenType::BeginOpenTag) => ParseState::State2,
            (ParseState::State4, XMLTokenType::Text) => ParseState::State5,
            (ParseState::State5, XMLTokenType::BeginCloseTag) => ParseState::State6,
            (ParseState::State6, XMLTokenType::Text) => ParseState::State7,
            (ParseState::State7, XMLTokenType::EndTag) => ParseState::State8,
            (ParseState::State8, XMLTokenType::BeginOpenTag) => ParseState::State2,
            (ParseState::State8, XMLTokenType::BeginCloseTag) => ParseState::State6,
            _ => ParseState::StateError,
        }
    }

    fn release(&mut self) {
        self.tag_stack.release();
        self.value_stack.release();
        self.element_stack.release();
        self.input.clear();
        self.position = 0;
        self.depth = 0;
        self.state = ParseState::State1;
    }
}

pub fn parse_xml_from_text(text: &str) -> Result<XMLElement, String> {
    let mut parser = XMLParser::new();
    parser.parse(text)
}
