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
        // No explicit release needed in safe Rust; clear children
        self.element.children.release();
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

        loop {
            let token = match self.get_next_token() {
                Some(t) => t,
                None => break,
            };

            // Skip empty TEXT tokens (trimmed away to nothing)
            if token.token_type == XMLTokenType::Text && token.data.is_none() {
                continue;
            }

            let new_state = Self::translate(self.state, &token.token_type);
            if new_state != ParseState::StateError {
                match self.state {
                    ParseState::State1 => {}
                    ParseState::State2 => {
                        if token.token_type == XMLTokenType::Text {
                            if let Some(d) = &token.data {
                                self.tag_stack.push_back(d.clone());
                                self.depth += 1;
                            }
                        }
                    }
                    ParseState::State3 => {}
                    ParseState::State4 => {
                        // STATE4 + TEXT -> push text on value_stack.
                        // STATE4 + BEGIN_OPEN_TAG -> nothing pushed.
                        if token.token_type == XMLTokenType::Text {
                            let d = token.data.clone().unwrap_or_default();
                            self.value_stack.push_back(d);
                        } else if token.token_type == XMLTokenType::BeginOpenTag {
                            // C pushes token->data (which is uninitialized for BEGIN_OPEN_TAG;
                            // but the value_stack is meant to track values).
                            // In the C code, only TEXT tokens carry data; BEGIN_OPEN_TAG token->data
                            // is uninitialized. But the C does `vector_push_back(parser->value_stack, token->data)`
                            // unconditionally in STATE4. This means in C, when STATE4 + BEGIN_OPEN_TAG
                            // happens, an uninitialized pointer is pushed. We'll push an empty string
                            // to represent this and rely on the parser logic to overwrite it.
                            // Actually looking more carefully: STATE4 + BEGIN_OPEN_TAG -> STATE2
                            // means new child opens; we don't have a value yet for the parent.
                            // The C code pushes uninitialized data; this is a bug in C but let's
                            // mimic by pushing an empty string.
                            self.value_stack.push_back(String::new());
                        }
                    }
                    ParseState::State5 => {}
                    ParseState::State6 => {
                        if token.token_type == XMLTokenType::Text {
                            // Assert that closing tag matches open tag
                            if let Some(d) = &token.data {
                                let top = self.tag_stack.top_back();
                                if let Some(t) = top {
                                    if t != d {
                                        return Err(format!(
                                            "Mismatched tag: expected {}, got {}",
                                            t, d
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    ParseState::State7 => {
                        if token.token_type == XMLTokenType::EndTag {
                            // Build XMLElement and put it on element_stack
                            let current_tag = self
                                .tag_stack
                                .top_back()
                                .cloned()
                                .unwrap_or_default();
                            let current_value = self
                                .value_stack
                                .top_back()
                                .cloned()
                                .unwrap_or_default();
                            let length = self.element_stack.size();
                            self.depth -= 1;

                            let mut current = XMLElement::new(current_tag, current_value);

                            // collect children: pop from element_stack while their depth > self.depth
                            // and prepend to current.children (push_front).
                            for _ in 0..length {
                                let elem_depth = match self.element_stack.top_back() {
                                    Some(e) => e.depth,
                                    None => break,
                                };
                                if elem_depth <= self.depth {
                                    break;
                                }
                                let popped = self.element_stack.pop_back().unwrap();
                                current.children.push_front(popped.element);
                            }

                            // push current onto element stack
                            let se = StackElement::new(current, self.depth);
                            self.element_stack.push_back(se);

                            self.tag_stack.pop_back();
                            self.value_stack.pop_back();
                        }
                    }
                    ParseState::State8 => {}
                    ParseState::StateError => {}
                }
            } else {
                return Err(format!("Parse error at state {:?}", self.state));
            }

            self.state = new_state;
        }

        // Top of element_stack should be the root
        match self.element_stack.pop_back() {
            Some(se) => Ok(se.element),
            None => Err("No root element".to_string()),
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
                        // We've encountered '<' after some text; back up and return text
                        self.position -= 1;
                        let to = self.position - 1;
                        return Some(self.get_text_token(begin_pos, to));
                    } else {
                        // Just consumed '<' as the first char
                        let next_char = if self.position < length {
                            bytes[self.position] as char
                        } else {
                            '\0'
                        };
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
                    }
                }
                END_TAG_TOKEN => {
                    if self.position > begin_pos + 1 {
                        self.position -= 1;
                        let to = self.position - 1;
                        return Some(self.get_text_token(begin_pos, to));
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

        // End of input - return remainder as text token
        let to = self.position - 1;
        Some(self.get_text_token(begin_pos, to))
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

        if to >= from && from < bytes.len() {
            let slice = &bytes[from..=to];
            let s = String::from_utf8_lossy(slice).to_string();
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
        // C state_translate table:
        // STATE1: BEGIN_OPEN_TAG -> STATE2
        // STATE2: TEXT -> STATE3
        // STATE3: END_TAG -> STATE4
        // STATE4: BEGIN_OPEN_TAG -> STATE2, TEXT -> STATE5
        // STATE5: BEGIN_CLOSE_TAG -> STATE6
        // STATE6: TEXT -> STATE7
        // STATE7: END_TAG -> STATE8
        // STATE8: BEGIN_OPEN_TAG -> STATE2, BEGIN_CLOSE_TAG -> STATE6
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
    }
}

pub fn parse_xml_from_text(text: &str) -> Result<XMLElement, String> {
    let mut parser = XMLParser::new();
    let result = parser.parse(text);
    parser.release();
    result
}
