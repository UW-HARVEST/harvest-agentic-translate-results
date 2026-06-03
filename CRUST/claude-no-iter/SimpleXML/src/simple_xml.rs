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
        // No-op: rely on Drop to release the contained XMLElement
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

fn is_ws(b: u8) -> bool {
    b == b' ' || b == b'\t' || b == b'\n' || b == b'\r'
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

            // Skip empty TEXT (matches C behavior of token->data == NULL)
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
                        if let Some(data) = token.data.clone() {
                            self.tag_stack.push_back(data);
                            self.depth += 1;
                        }
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
                        if let Some(data) = &token.data {
                            let top_tag =
                                self.tag_stack.top_back().cloned().unwrap_or_default();
                            if data != &top_tag {
                                return Err(format!(
                                    "mismatched tag: expected '{}' got '{}'",
                                    top_tag, data
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

                        // Find children of current elem.  Pop StackElements with
                        // depth > current depth from the back of element_stack and
                        // push them to the front of `current.children` (so original
                        // order is preserved).
                        for _ in 0..length {
                            let should_pop = match self.element_stack.top_back() {
                                Some(elem) => elem.depth > se_depth,
                                None => false,
                            };
                            if !should_pop {
                                break;
                            }
                            if let Some(popped) = self.element_stack.pop_back() {
                                current.children.push_front(popped.element);
                            } else {
                                break;
                            }
                        }

                        // Push current element onto the element stack
                        self.element_stack
                            .push_back(StackElement::new(current, se_depth));

                        self.tag_stack.pop_back();
                        self.value_stack.pop_back();
                    }
                }
                ParseState::State8 => {}
                ParseState::StateError => {}
            }

            self.state = new_state;
        }

        // Take the topmost element from the element_stack as the root
        match self.element_stack.pop_back() {
            Some(se) => Ok(se.element),
            None => Err("no XML elements parsed".to_string()),
        }
    }

    fn get_next_token(&mut self) -> Option<XMLToken> {
        let length = self.input.len();
        let begin_pos = self.position;

        if begin_pos >= length {
            return None;
        }

        let bytes = self.input.as_bytes();

        while self.position < length {
            let ch = bytes[self.position];
            self.position += 1;

            match ch {
                b'<' => {
                    if self.position > begin_pos + 1 {
                        // we already consumed some text before the '<'
                        self.position -= 1;
                        return Some(self.get_text_token(begin_pos, self.position - 1));
                    } else {
                        // `<` was the first character of the token
                        if self.position < length && bytes[self.position] == b'/' {
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
                b'>' => {
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
                _ => {
                    // continue scanning
                }
            }
        }

        // Reached end of input, return whatever text we accumulated
        if self.position == 0 {
            return None;
        }
        Some(self.get_text_token(begin_pos, self.position - 1))
    }

    fn get_text_token(&self, mut from: usize, mut to: usize) -> XMLToken {
        let bytes = self.input.as_bytes();
        let len = bytes.len();

        // Trim leading whitespace
        while from <= to && from < len && is_ws(bytes[from]) {
            from += 1;
        }

        // Trim trailing whitespace
        while to >= from && to < len && is_ws(bytes[to]) {
            if to == 0 {
                // Avoid usize underflow.  Force the "no data" branch below.
                from = 1;
                break;
            }
            to -= 1;
        }

        let data = if to >= from && to < len {
            Some(self.input[from..=to].to_string())
        } else {
            None
        };

        XMLToken {
            token_type: XMLTokenType::Text,
            data,
        }
    }

    fn translate(state: ParseState, token: &XMLTokenType) -> ParseState {
        let token_idx = match token {
            XMLTokenType::BeginOpenTag => 0,
            XMLTokenType::BeginCloseTag => 1,
            XMLTokenType::EndTag => 2,
            XMLTokenType::Text => 3,
        };
        let state_idx = match state {
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
        let table: [[ParseState; 4]; 8] = [
            // BeginOpenTag    BeginCloseTag       EndTag             Text
            [ParseState::State2, E,                 E,                 E],
            [E,                 E,                 E,                 ParseState::State3],
            [E,                 E,                 ParseState::State4, E],
            [ParseState::State2, E,                 E,                 ParseState::State5],
            [E,                 ParseState::State6, E,                 E],
            [E,                 E,                 E,                 ParseState::State7],
            [E,                 E,                 ParseState::State8, E],
            [ParseState::State2, ParseState::State6, E,                 E],
        ];

        table[state_idx][token_idx]
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

// Suppress unused warnings for constants that are part of the public surface.
#[allow(dead_code)]
const _UNUSED: (char, char, char) = (BEGIN_TAG_TOKEN, END_TAG_TOKEN, SPLASH_TOKEN);
