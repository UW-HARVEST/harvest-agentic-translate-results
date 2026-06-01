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
        // No-op in safe Rust; storage is released by Drop
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

            // Skip empty TEXT tokens (data == None)
            if token.token_type == XMLTokenType::Text && token.data.is_none() {
                continue;
            }

            let next_state = Self::translate(self.state, &token.token_type);
            if next_state == ParseState::StateError {
                return Err("error while parsing".to_string());
            }

            match self.state {
                ParseState::State1 => {}
                ParseState::State2 => {
                    if token.token_type == XMLTokenType::Text {
                        let data = token.data.clone().unwrap_or_default();
                        self.tag_stack.push_back(data);
                        self.depth += 1;
                    }
                }
                ParseState::State3 => {}
                ParseState::State4 => {
                    let data = token.data.clone().unwrap_or_default();
                    self.value_stack.push_back(data);
                }
                ParseState::State5 => {}
                ParseState::State6 => {
                    if token.token_type == XMLTokenType::Text {
                        if let (Some(top), Some(d)) =
                            (self.tag_stack.top_back(), token.data.as_ref())
                        {
                            if top != d {
                                return Err(format!(
                                    "Mismatched closing tag: '{}' vs '{}'",
                                    d, top
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
                        let length = self.element_stack.size();
                        if self.depth > 0 {
                            self.depth -= 1;
                        }

                        let mut current = XMLElement::new(current_tag, current_value);
                        let se_depth = self.depth;

                        // Find children of current elem
                        for _ in 0..length {
                            let should_pop = match self.element_stack.top_back() {
                                Some(top) => top.depth > se_depth,
                                None => false,
                            };
                            if !should_pop {
                                break;
                            }
                            if let Some(elem) = self.element_stack.pop_back() {
                                current.children.push_front(elem.element);
                            }
                        }

                        self.element_stack
                            .push_back(StackElement::new(current, se_depth));

                        self.tag_stack.pop_back();
                        self.value_stack.pop_back();
                    }
                }
                ParseState::State8 => {}
                ParseState::StateError => {}
            }

            self.state = next_state;
        }

        match self.element_stack.pop_back() {
            Some(se) => Ok(se.element),
            None => Err("No element parsed".to_string()),
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
            let ch = bytes[self.position] as char;
            self.position += 1;

            match ch {
                BEGIN_TAG_TOKEN => {
                    if self.position > begin_pos + 1 {
                        // There was text before '<'; back up and return a text token
                        self.position -= 1;
                        return Some(self.get_text_token(begin_pos, self.position - 1));
                    } else {
                        // '<' was the very first character
                        if self.position < length {
                            let next_char = bytes[self.position] as char;
                            if next_char == SPLASH_TOKEN {
                                self.position += 1;
                                return Some(XMLToken {
                                    token_type: XMLTokenType::BeginCloseTag,
                                    data: None,
                                });
                            }
                        }
                        return Some(XMLToken {
                            token_type: XMLTokenType::BeginOpenTag,
                            data: None,
                        });
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
        let len = bytes.len();

        // Trim leading spaces (and other whitespace common in formatted XML)
        while from < len && Self::is_xml_space(bytes[from]) {
            from += 1;
        }
        // Trim trailing spaces — but be careful not to underflow `to`.
        // If `from > to` already, skip trimming.
        if from <= to && to < len {
            while to < len && Self::is_xml_space(bytes[to]) {
                if to == 0 || to < from {
                    // If we'd underflow or move past `from`, stop and signal empty
                    return XMLToken {
                        token_type: XMLTokenType::Text,
                        data: None,
                    };
                }
                to -= 1;
            }
        }

        if from <= to && to < len {
            let s = std::str::from_utf8(&bytes[from..=to])
                .unwrap_or("")
                .to_string();
            if s.is_empty() {
                XMLToken {
                    token_type: XMLTokenType::Text,
                    data: None,
                }
            } else {
                XMLToken {
                    token_type: XMLTokenType::Text,
                    data: Some(s),
                }
            }
        } else {
            XMLToken {
                token_type: XMLTokenType::Text,
                data: None,
            }
        }
    }

    fn is_xml_space(b: u8) -> bool {
        // The C code only trims spaces, but inputs in the Rust test use raw
        // multiline strings with newlines and indentation. Treat all common
        // ASCII whitespace as trimmable so unmodified test inputs work.
        b == b' ' || b == b'\t' || b == b'\n' || b == b'\r'
    }

    fn translate(state: ParseState, token: &XMLTokenType) -> ParseState {
        // STATE_TRANSLATE[state][token_type]
        // Columns: BEGIN_OPEN_TAG, BEGIN_CLOSE_TAG, END_TAG, TEXT
        let row: [ParseState; 4] = match state {
            ParseState::State1 => [
                ParseState::State2,
                ParseState::StateError,
                ParseState::StateError,
                ParseState::StateError,
            ],
            ParseState::State2 => [
                ParseState::StateError,
                ParseState::StateError,
                ParseState::StateError,
                ParseState::State3,
            ],
            ParseState::State3 => [
                ParseState::StateError,
                ParseState::StateError,
                ParseState::State4,
                ParseState::StateError,
            ],
            ParseState::State4 => [
                ParseState::State2,
                ParseState::StateError,
                ParseState::StateError,
                ParseState::State5,
            ],
            ParseState::State5 => [
                ParseState::StateError,
                ParseState::State6,
                ParseState::StateError,
                ParseState::StateError,
            ],
            ParseState::State6 => [
                ParseState::StateError,
                ParseState::StateError,
                ParseState::StateError,
                ParseState::State7,
            ],
            ParseState::State7 => [
                ParseState::StateError,
                ParseState::StateError,
                ParseState::State8,
                ParseState::StateError,
            ],
            ParseState::State8 => [
                ParseState::State2,
                ParseState::State6,
                ParseState::StateError,
                ParseState::StateError,
            ],
            ParseState::StateError => [ParseState::StateError; 4],
        };
        let idx = match token {
            XMLTokenType::BeginOpenTag => 0,
            XMLTokenType::BeginCloseTag => 1,
            XMLTokenType::EndTag => 2,
            XMLTokenType::Text => 3,
        };
        row[idx]
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
