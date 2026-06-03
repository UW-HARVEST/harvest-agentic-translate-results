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
        // Nothing to do; Rust handles memory automatically.
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

            // Skip empty text tokens.
            if token.token_type == XMLTokenType::Text && token.data.is_none() {
                continue;
            }

            let new_state = Self::translate(self.state, &token.token_type);
            if new_state == ParseState::StateError {
                return Err(format!(
                    "error while parsing at position {}",
                    self.position
                ));
            }

            match self.state {
                ParseState::State1 => {}
                ParseState::State2 => {
                    if token.token_type == XMLTokenType::Text {
                        let tag_data = token.data.clone().unwrap_or_default();
                        self.tag_stack.push_back(tag_data);
                        // Push placeholder empty value so the stacks stay in sync.
                        self.value_stack.push_back(String::new());
                        self.depth += 1;
                    }
                }
                ParseState::State3 => {}
                ParseState::State4 => {
                    if token.token_type == XMLTokenType::Text {
                        // Replace the placeholder value with the actual text.
                        let value_data = token.data.clone().unwrap_or_default();
                        let idx = self.value_stack.size - 1;
                        self.value_stack.data[idx] = value_data;
                    }
                }
                ParseState::State5 => {}
                ParseState::State6 => {
                    if token.token_type == XMLTokenType::Text {
                        let close_tag = token.data.clone().unwrap_or_default();
                        let top = self
                            .tag_stack
                            .top_back()
                            .ok_or_else(|| "tag stack empty".to_string())?;
                        if &close_tag != top {
                            return Err(format!(
                                "mismatched closing tag: expected {} got {}",
                                top, close_tag
                            ));
                        }
                    }
                }
                ParseState::State7 => {
                    if token.token_type == XMLTokenType::EndTag {
                        let current_tag = self
                            .tag_stack
                            .top_back()
                            .cloned()
                            .ok_or_else(|| "tag stack empty".to_string())?;
                        let current_value = self
                            .value_stack
                            .top_back()
                            .cloned()
                            .unwrap_or_default();

                        let length = self.element_stack.size();
                        if self.depth == 0 {
                            return Err("depth underflow".to_string());
                        }
                        self.depth -= 1;

                        let mut current = XMLElement::new(current_tag, current_value);
                        let se_depth = self.depth;

                        // Pull children: pop from element_stack while top depth > se_depth.
                        // Use push_front so original sibling order is preserved.
                        for _ in 0..length {
                            let top_depth = match self.element_stack.top_back() {
                                Some(s) => s.depth,
                                None => break,
                            };
                            if top_depth <= se_depth {
                                break;
                            }
                            let child_se = self.element_stack.pop_back().unwrap();
                            current.children.push_front(child_se.element);
                        }

                        let se = StackElement::new(current, se_depth);
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

        let top_se = self
            .element_stack
            .pop_back()
            .ok_or_else(|| "no element produced".to_string())?;
        Ok(top_se.element)
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
                        // We had accumulated text before this `<`.
                        self.position -= 1;
                        return Some(self.get_text_token(begin_pos, self.position - 1));
                    } else {
                        // Just consumed `<` at position begin_pos.
                        if self.position < length
                            && bytes[self.position] as char == SPLASH_TOKEN
                        {
                            let token = XMLToken {
                                token_type: XMLTokenType::BeginCloseTag,
                                data: None,
                            };
                            self.position += 1;
                            return Some(token);
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

        // End of input reached while accumulating text.
        Some(self.get_text_token(begin_pos, self.position - 1))
    }

    fn get_text_token(&self, mut from: usize, mut to: usize) -> XMLToken {
        let bytes = self.input.as_bytes();

        // Trim leading whitespace.
        while from <= to && (bytes[from] as char).is_whitespace() {
            from += 1;
        }
        // Trim trailing whitespace.
        while to >= from && (bytes[to] as char).is_whitespace() {
            if to == 0 {
                // Avoid underflow.
                if (bytes[to] as char).is_whitespace() && to < from {
                    break;
                }
                break;
            }
            to -= 1;
        }

        if to >= from {
            let slice = &bytes[from..=to];
            // Convert byte slice back to String. Input is valid UTF-8.
            let data = String::from_utf8_lossy(slice).into_owned();
            XMLToken {
                token_type: XMLTokenType::Text,
                data: Some(data),
            }
        } else {
            XMLToken {
                token_type: XMLTokenType::Text,
                data: None,
            }
        }
    }

    fn translate(state: ParseState, token: &XMLTokenType) -> ParseState {
        // Index: 0 = BeginOpenTag, 1 = BeginCloseTag, 2 = EndTag, 3 = Text
        let token_idx = match token {
            XMLTokenType::BeginOpenTag => 0,
            XMLTokenType::BeginCloseTag => 1,
            XMLTokenType::EndTag => 2,
            XMLTokenType::Text => 3,
        };

        // Translation table mirroring the C `state_translate` array.
        let table: [[ParseState; 4]; 8] = [
            // BeginOpenTag, BeginCloseTag, EndTag, Text
            [
                ParseState::State2,
                ParseState::StateError,
                ParseState::StateError,
                ParseState::StateError,
            ], // STATE1
            [
                ParseState::StateError,
                ParseState::StateError,
                ParseState::StateError,
                ParseState::State3,
            ], // STATE2
            [
                ParseState::StateError,
                ParseState::StateError,
                ParseState::State4,
                ParseState::StateError,
            ], // STATE3
            [
                ParseState::State2,
                ParseState::StateError,
                ParseState::StateError,
                ParseState::State5,
            ], // STATE4
            [
                ParseState::StateError,
                ParseState::State6,
                ParseState::StateError,
                ParseState::StateError,
            ], // STATE5
            [
                ParseState::StateError,
                ParseState::StateError,
                ParseState::StateError,
                ParseState::State7,
            ], // STATE6
            [
                ParseState::StateError,
                ParseState::StateError,
                ParseState::State8,
                ParseState::StateError,
            ], // STATE7
            [
                ParseState::State2,
                ParseState::State6,
                ParseState::StateError,
                ParseState::StateError,
            ], // STATE8
        ];

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
    let result = parser.parse(text);
    parser.release();
    result
}
