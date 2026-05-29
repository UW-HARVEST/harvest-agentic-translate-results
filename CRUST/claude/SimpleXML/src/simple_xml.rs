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
        // No-op: Drop handles cleanup
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

            // Skip empty TEXT tokens (whitespace-only)
            if matches!(token.token_type, XMLTokenType::Text) && token.data.is_none() {
                continue;
            }

            let next_state = Self::translate(self.state, &token.token_type);
            if matches!(next_state, ParseState::StateError) {
                return Err("error while parsing".to_string());
            }

            match self.state {
                ParseState::State1 => {}
                ParseState::State2 => {
                    if matches!(token.token_type, XMLTokenType::Text) {
                        let data = token.data.clone().unwrap_or_default();
                        self.tag_stack.push_back(data);
                        self.depth += 1;
                    }
                }
                ParseState::State3 => {}
                ParseState::State4 => {
                    // C unconditionally pushes token->data here. If the token
                    // is BEGIN_OPEN_TAG (no data), we push an empty string as
                    // a placeholder so that pops in STATE7 stay balanced.
                    if matches!(token.token_type, XMLTokenType::Text) {
                        let data = token.data.clone().unwrap_or_default();
                        self.value_stack.push_back(data);
                    } else {
                        self.value_stack.push_back(String::new());
                    }
                }
                ParseState::State5 => {}
                ParseState::State6 => {
                    if matches!(token.token_type, XMLTokenType::Text) {
                        let top = self.tag_stack.top_back().cloned().unwrap_or_default();
                        let tok_data = token.data.clone().unwrap_or_default();
                        if tok_data != top {
                            return Err(format!(
                                "tag mismatch: expected {}, got {}",
                                top, tok_data
                            ));
                        }
                    }
                }
                ParseState::State7 => {
                    if matches!(token.token_type, XMLTokenType::EndTag) {
                        let current_tag = self.tag_stack.pop_back().unwrap_or_default();
                        let current_value = self.value_stack.pop_back().unwrap_or_default();
                        let length = self.element_stack.size();
                        if self.depth > 0 {
                            self.depth -= 1;
                        }

                        let mut current = XMLElement::new(current_tag, current_value);
                        let se_depth = self.depth;

                        // Find children of current element: pop from element_stack
                        // while the top has depth strictly greater than ours.
                        for _ in 0..length {
                            let top_depth = match self.element_stack.top_back() {
                                Some(s) => s.depth,
                                None => break,
                            };
                            if top_depth <= se_depth {
                                break;
                            }
                            if let Some(child_se) = self.element_stack.pop_back() {
                                current.children.push_front(child_se.element);
                            } else {
                                break;
                            }
                        }

                        self.element_stack
                            .push_back(StackElement::new(current, se_depth));
                    }
                }
                ParseState::State8 => {}
                ParseState::StateError => {}
            }

            self.state = next_state;
        }

        match self.element_stack.pop_back() {
            Some(se) => Ok(se.element),
            None => Err("no element parsed".to_string()),
        }
    }

    fn get_next_token(&mut self) -> Option<XMLToken> {
        let length = self.input.len();
        let begin_pos = self.position;

        if begin_pos >= length {
            return None;
        }

        while self.position < length {
            let ch = self.input.as_bytes()[self.position] as char;
            self.position += 1;

            match ch {
                BEGIN_TAG_TOKEN => {
                    if self.position > begin_pos + 1 {
                        self.position -= 1;
                        return Some(self.get_text_token(begin_pos, self.position - 1));
                    } else {
                        // Look at next char to decide between BEGIN_OPEN_TAG
                        // and BEGIN_CLOSE_TAG.
                        let next_char = if self.position < length {
                            self.input.as_bytes()[self.position] as char
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
                        return Some(self.get_text_token(begin_pos, self.position - 1));
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
        // Trim leading whitespace
        while from <= to && (bytes[from] as char).is_whitespace() {
            from += 1;
        }
        // Trim trailing whitespace; guard against underflow when to == 0
        while to >= from && (bytes[to] as char).is_whitespace() {
            if to == 0 {
                // Empty after trimming
                return XMLToken {
                    token_type: XMLTokenType::Text,
                    data: None,
                };
            }
            to -= 1;
        }

        if to >= from {
            let slice = &bytes[from..=to];
            match std::str::from_utf8(slice) {
                Ok(s) => XMLToken {
                    token_type: XMLTokenType::Text,
                    data: Some(s.to_string()),
                },
                Err(_) => XMLToken {
                    token_type: XMLTokenType::Text,
                    data: None,
                },
            }
        } else {
            XMLToken {
                token_type: XMLTokenType::Text,
                data: None,
            }
        }
    }

    fn translate(state: ParseState, token: &XMLTokenType) -> ParseState {
        let col = match token {
            XMLTokenType::BeginOpenTag => 0,
            XMLTokenType::BeginCloseTag => 1,
            XMLTokenType::EndTag => 2,
            XMLTokenType::Text => 3,
        };
        let row = match state {
            ParseState::State1 => 0,
            ParseState::State2 => 1,
            ParseState::State3 => 2,
            ParseState::State4 => 3,
            ParseState::State5 => 4,
            ParseState::State6 => 5,
            ParseState::State7 => 6,
            ParseState::State8 => 7,
            ParseState::StateError => return ParseState::StateError,
        };
        const E: ParseState = ParseState::StateError;
        const TABLE: [[ParseState; 4]; 8] = [
            // BEGIN_OPEN_TAG       BEGIN_CLOSE_TAG       END_TAG               TEXT
            [ParseState::State2,    E,                    E,                    E                  ], // STATE1
            [E,                     E,                    E,                    ParseState::State3 ], // STATE2
            [E,                     E,                    ParseState::State4,   E                  ], // STATE3
            [ParseState::State2,    E,                    E,                    ParseState::State5 ], // STATE4
            [E,                     ParseState::State6,   E,                    E                  ], // STATE5
            [E,                     E,                    E,                    ParseState::State7 ], // STATE6
            [E,                     E,                    ParseState::State8,   E                  ], // STATE7
            [ParseState::State2,    ParseState::State6,   E,                    E                  ], // STATE8
        ];
        TABLE[row][col]
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
