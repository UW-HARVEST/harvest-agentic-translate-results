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
    BeginOpenTag,  // "<"
    BeginCloseTag, // "</"
    EndTag,        // ">"
    Text,          // non-empty, trimmed text
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
        // Releases handled automatically by Rust drop semantics.
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

            // Skip empty (post-trim) TEXT tokens, matching C's `data == NULL` filter.
            if token.token_type == XMLTokenType::Text && token.data.is_none() {
                continue;
            }

            let new_state = Self::translate(self.state, &token.token_type);

            if new_state == ParseState::StateError {
                return Err("error while parsing".to_string());
            }

            match self.state {
                ParseState::State1 => {}
                ParseState::State2 => {
                    if token.token_type == XMLTokenType::Text {
                        if let Some(data) = token.data.as_ref() {
                            self.tag_stack.push_back(data.clone());
                            self.depth += 1;
                        }
                    }
                }
                ParseState::State3 => {}
                ParseState::State4 => {
                    // C pushes token->data unconditionally (could be NULL).
                    let data = token.data.clone().unwrap_or_default();
                    self.value_stack.push_back(data);
                }
                ParseState::State5 => {}
                ParseState::State6 => {
                    if token.token_type == XMLTokenType::Text {
                        if let (Some(data), Some(top)) =
                            (token.data.as_ref(), self.tag_stack.top_back())
                        {
                            if data != top {
                                return Err(format!(
                                    "tag mismatch: closing '{}' but expected '{}'",
                                    data, top
                                ));
                            }
                        }
                    }
                }
                ParseState::State7 => {
                    if token.token_type == XMLTokenType::EndTag {
                        let current_tag =
                            self.tag_stack.top_back().cloned().unwrap_or_default();
                        let current_value =
                            self.value_stack.top_back().cloned().unwrap_or_default();
                        if self.depth > 0 {
                            self.depth -= 1;
                        }
                        let new_depth = self.depth;

                        let mut current = XMLElement::new(current_tag, current_value);

                        // Collect any children whose depth is greater than new_depth.
                        while self
                            .element_stack
                            .top_back()
                            .map_or(false, |t| t.depth > new_depth)
                        {
                            let elem = self.element_stack.pop_back().unwrap();
                            current.children.push_front(elem.element);
                        }

                        let se = StackElement::new(current, new_depth);
                        self.element_stack.push_back(se);

                        self.tag_stack.pop_back();
                        self.value_stack.pop_back();
                    }
                }
                ParseState::State8 => {}
                ParseState::StateError => {}
            }

            self.state = new_state;
        }

        if let Some(se) = self.element_stack.pop_back() {
            Ok(se.element)
        } else {
            Err("no element parsed".to_string())
        }
    }

    fn get_next_token(&mut self) -> Option<XMLToken> {
        let begin_pos = self.position;
        let bytes = self.input.as_bytes();
        let length = bytes.len();

        if begin_pos >= length {
            return None;
        }

        while self.position < length {
            let ch = bytes[self.position];
            self.position += 1;

            if ch == BEGIN_TAG_TOKEN as u8 {
                if self.position > begin_pos + 1 {
                    self.position -= 1;
                    return Some(self.get_text_token(begin_pos, self.position - 1));
                } else if self.position < length && bytes[self.position] == SPLASH_TOKEN as u8 {
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
            } else if ch == END_TAG_TOKEN as u8 {
                if self.position > begin_pos + 1 {
                    self.position -= 1;
                    return Some(self.get_text_token(begin_pos, self.position - 1));
                } else {
                    return Some(XMLToken {
                        token_type: XMLTokenType::EndTag,
                        data: None,
                    });
                }
            }
        }

        Some(self.get_text_token(begin_pos, self.position - 1))
    }

    fn get_text_token(&self, mut from: usize, mut to: usize) -> XMLToken {
        let bytes = self.input.as_bytes();

        if to >= bytes.len() {
            return XMLToken {
                token_type: XMLTokenType::Text,
                data: None,
            };
        }

        // Trim leading whitespace.
        while from <= to && bytes[from].is_ascii_whitespace() {
            if from == to {
                // Range will become empty next iteration.
                return XMLToken {
                    token_type: XMLTokenType::Text,
                    data: None,
                };
            }
            from += 1;
        }
        // Trim trailing whitespace (we know `to >= from` and bytes[from] non-whitespace).
        while to > from && bytes[to].is_ascii_whitespace() {
            to -= 1;
        }

        if to >= from && !bytes[from].is_ascii_whitespace() {
            let text =
                std::str::from_utf8(&bytes[from..=to]).unwrap_or("").to_string();
            if text.is_empty() {
                XMLToken {
                    token_type: XMLTokenType::Text,
                    data: None,
                }
            } else {
                XMLToken {
                    token_type: XMLTokenType::Text,
                    data: Some(text),
                }
            }
        } else {
            XMLToken {
                token_type: XMLTokenType::Text,
                data: None,
            }
        }
    }

    fn translate(state: ParseState, token: &XMLTokenType) -> ParseState {
        use ParseState::*;
        use XMLTokenType::*;
        match (state, token) {
            (State1, BeginOpenTag) => State2,
            (State2, Text) => State3,
            (State3, EndTag) => State4,
            (State4, BeginOpenTag) => State2,
            (State4, Text) => State5,
            (State5, BeginCloseTag) => State6,
            (State6, Text) => State7,
            (State7, EndTag) => State8,
            (State8, BeginOpenTag) => State2,
            (State8, BeginCloseTag) => State6,
            _ => StateError,
        }
    }

    fn release(&mut self) {
        self.tag_stack.release();
        self.value_stack.release();
        self.element_stack.release();
    }
}

pub fn parse_xml_from_text(text: &str) -> Result<XMLElement, String> {
    let mut parser = XMLParser::new();
    parser.parse(text)
}
