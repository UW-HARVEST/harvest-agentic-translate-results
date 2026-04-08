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
        // no-op in Rust, ownership handles cleanup
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
        self.state = ParseState::State1;

        loop {
            let token = match self.get_next_token() {
                Some(t) => t,
                None => break,
            };
            // skip TEXT tokens with no data
            if token.token_type == XMLTokenType::Text && token.data.is_none() {
                continue;
            }

            let new_state = Self::translate(self.state, &token.token_type);
            if new_state != ParseState::StateError {
                match self.state {
                    ParseState::State1 => {}
                    ParseState::State2 => {
                        if token.token_type == XMLTokenType::Text {
                            self.tag_stack.push_back(token.data.unwrap());
                            self.depth += 1;
                        }
                    }
                    ParseState::State3 => {}
                    ParseState::State4 => {
                        // push text value (or empty) between open tag end and next token
                        self.value_stack.push_back(token.data.unwrap_or_default());
                    }
                    ParseState::State5 => {}
                    ParseState::State6 => {
                        if token.token_type == XMLTokenType::Text {
                            let top_tag = self.tag_stack.top_back().unwrap();
                            let close_tag = token.data.as_ref().unwrap();
                            assert_eq!(close_tag, top_tag);
                        }
                    }
                    ParseState::State7 => {
                        if token.token_type == XMLTokenType::EndTag {
                            let current_tag = self.tag_stack.top_back().unwrap().clone();
                            let current_value = self.value_stack.top_back().unwrap().clone();
                            self.depth -= 1;

                            let mut current = XMLElement::new(current_tag, current_value);
                            let current_depth = self.depth;

                            // find children: pop from element_stack while depth > current_depth
                            let length = self.element_stack.size();
                            let mut children_rev = Vec::new();
                            for _ in 0..length {
                                let top = self.element_stack.top_back().unwrap();
                                if top.depth <= current_depth {
                                    break;
                                }
                                let se = self.element_stack.pop_back().unwrap();
                                children_rev.push(se.element);
                            }
                            // push_front in C means children end up in original order
                            for child in children_rev.into_iter().rev() {
                                current.children.push_back(child);
                            }

                            let se = StackElement::new(current, current_depth);
                            self.element_stack.push_back(se);

                            self.tag_stack.pop_back();
                            self.value_stack.pop_back();
                        }
                    }
                    ParseState::State8 => {}
                    _ => {}
                }
            }

            if new_state == ParseState::StateError {
                return Err("error while parsing".to_string());
            }
            self.state = new_state;
        }

        let se = self.element_stack.pop_back().ok_or("empty element stack")?;
        Ok(se.element)
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
                        self.position -= 1;
                        return Some(self.get_text_token(begin_pos, self.position - 1));
                    } else {
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
        // trim leading spaces
        while from <= to && bytes[from] == b' ' {
            from += 1;
        }
        // trim trailing spaces
        while to > from && bytes[to] == b' ' {
            to -= 1;
        }

        if to >= from && from <= to {
            let data = self.input[from..=to].to_string();
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
        // no-op in Rust
    }
}

pub fn parse_xml_from_text(text: &str) -> Result<XMLElement, String> {
    let mut parser = XMLParser::new();
    parser.parse(text)
}
