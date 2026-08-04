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
        // Children are owned and will be dropped automatically when this is
        // dropped. We mirror the C `StackElement_release` by clearing the
        // children vector explicitly.
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
        self.tag_stack = Vector::new(8);
        self.value_stack = Vector::new(8);
        self.element_stack = Vector::new(8);

        loop {
            let token = match self.get_next_token() {
                Some(t) => t,
                None => break,
            };

            // Skip TEXT tokens with no data (whitespace only)
            if token.token_type == XMLTokenType::Text && token.data.is_none() {
                continue;
            }

            let next_state = Self::translate(self.state, &token.token_type);
            if next_state == ParseState::StateError {
                return Err(format!(
                    "error while parsing at position {}",
                    self.position
                ));
            }

            // Process actions for the current state, mirroring the switch in
            // the C `parse_xml_from_text` routine.
            let token_type = token.token_type.clone();
            let mut token_data = token.data;

            match self.state {
                ParseState::State1 => {}
                ParseState::State2 => {
                    if token_type == XMLTokenType::Text {
                        if let Some(data) = token_data.take() {
                            self.tag_stack.push_back(data);
                            self.depth += 1;
                        }
                    }
                }
                ParseState::State3 => {}
                ParseState::State4 => {
                    // The C code unconditionally pushes `token->data` onto the
                    // value_stack when leaving STATE4. For BEGIN_OPEN_TAG the
                    // data field is uninitialized memory (later overwritten in
                    // STATE7 for the parent element's value). We mirror the
                    // behavior by pushing an empty string in that case so
                    // value_stack stays aligned with tag_stack.
                    match token_type {
                        XMLTokenType::Text => {
                            self.value_stack
                                .push_back(token_data.take().unwrap_or_default());
                        }
                        XMLTokenType::BeginOpenTag => {
                            self.value_stack.push_back(String::new());
                        }
                        _ => {}
                    }
                }
                ParseState::State5 => {}
                ParseState::State6 => {
                    if token_type == XMLTokenType::Text {
                        if let Some(ref data) = token_data {
                            match self.tag_stack.top_back() {
                                Some(top) if data != top => {
                                    return Err(format!(
                                        "mismatched closing tag: '{}' vs '{}'",
                                        data, top
                                    ));
                                }
                                None => {
                                    return Err(
                                        "closing tag without matching open tag".to_string()
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                }
                ParseState::State7 => {
                    if token_type == XMLTokenType::EndTag {
                        let current_tag =
                            self.tag_stack.pop_back().unwrap_or_default();
                        let current_value =
                            self.value_stack.pop_back().unwrap_or_default();
                        if self.depth > 0 {
                            self.depth -= 1;
                        }

                        let mut current = XMLElement::new(current_tag, current_value);
                        let se_depth = self.depth;

                        // Pop any items off the element stack whose depth
                        // exceeds the current depth — those are children of
                        // the element we are now closing. Use push_front so
                        // they appear in source order.
                        loop {
                            let top_depth =
                                self.element_stack.top_back().map(|se| se.depth);
                            match top_depth {
                                Some(d) if d > se_depth => {
                                    let popped =
                                        self.element_stack.pop_back().unwrap();
                                    current.children.push_front(popped.element);
                                }
                                _ => break,
                            }
                        }

                        let se = StackElement::new(current, se_depth);
                        self.element_stack.push_back(se);
                    }
                }
                ParseState::State8 => {}
                ParseState::StateError => {}
            }

            self.state = next_state;
        }

        // The fully-parsed root element is the only StackElement still on the
        // element_stack at end-of-input.
        match self.element_stack.pop_back() {
            Some(stack_elem) => Ok(stack_elem.element),
            None => Err("no XML element parsed".to_string()),
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
                        // There was preceding text, return it first; rewind
                        // one position so the next call sees '<' again.
                        self.position -= 1;
                        return Some(self.get_text_token(begin_pos, self.position - 1));
                    } else if self.position < length {
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

        // Reached end of input while reading text.
        if self.position == 0 {
            return None;
        }
        Some(self.get_text_token(begin_pos, self.position - 1))
    }

    fn get_text_token(&self, mut from: usize, mut to: usize) -> XMLToken {
        let bytes = self.input.as_bytes();
        let length = bytes.len();

        // Trim leading spaces.
        while from < length && from <= to && bytes[from] == b' ' {
            from += 1;
        }
        // Trim trailing spaces, guarding against `usize` underflow.
        while to >= from && to < length && bytes[to] == b' ' {
            if to == 0 {
                // Range collapses to nothing.
                return XMLToken {
                    token_type: XMLTokenType::Text,
                    data: None,
                };
            }
            to -= 1;
        }

        if to >= from && to < length {
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
        use ParseState::*;
        use XMLTokenType::*;

        // Replicates the `state_translate` table from the C implementation.
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
