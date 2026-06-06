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
        // No-op in safe Rust; resources are released when dropped.
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
        self.depth = 0;

        loop {
            let token = match self.get_next_token() {
                Some(t) => t,
                None => break,
            };

            // Skip empty text tokens (data is None)
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
                        if let Some(data) = token.data {
                            self.tag_stack.push_back(data);
                            self.depth += 1;
                        }
                    }
                }
                ParseState::State3 => {}
                ParseState::State4 => {
                    // Mirror the C behavior: always push token->data to value_stack.
                    // For non-Text tokens (e.g., BeginOpenTag), data is None and we
                    // push an empty string as a placeholder so the stack stays in
                    // sync with the tag_stack.
                    let data = token.data.clone().unwrap_or_default();
                    self.value_stack.push_back(data);
                }
                ParseState::State5 => {}
                ParseState::State6 => {
                    if token.token_type == XMLTokenType::Text {
                        if let Some(ref data) = token.data {
                            let top = self
                                .tag_stack
                                .top_back()
                                .ok_or_else(|| "tag stack empty".to_string())?;
                            if data != top {
                                return Err(format!(
                                    "mismatched closing tag: expected {}, got {}",
                                    top, data
                                ));
                            }
                        }
                    }
                }
                ParseState::State7 => {
                    if token.token_type == XMLTokenType::EndTag {
                        let current_tag = self
                            .tag_stack
                            .pop_back()
                            .ok_or_else(|| "tag stack empty".to_string())?;
                        let current_value = self
                            .value_stack
                            .pop_back()
                            .ok_or_else(|| "value stack empty".to_string())?;
                        if self.depth == 0 {
                            return Err("invalid depth".to_string());
                        }
                        self.depth -= 1;

                        let mut current = XMLElement::new(current_tag, current_value);
                        let current_depth = self.depth;

                        // Pop and collect children whose depth > current_depth
                        let mut popped: Vec<StackElement> = Vec::new();
                        loop {
                            let should_pop = match self.element_stack.top_back() {
                                Some(elem) => elem.depth > current_depth,
                                None => false,
                            };
                            if !should_pop {
                                break;
                            }
                            let p = self.element_stack.pop_back().unwrap();
                            popped.push(p);
                        }
                        // C does push_front for each popped (in pop order),
                        // so result order in children is reverse of pop order.
                        // popped[0] was top of element_stack (last pushed, latest sibling),
                        // pushed front first => ends up at index N-1 (last child).
                        // popped[N-1] was earliest sibling, pushed front last => index 0.
                        // To get the same result with push_back, push in reverse pop order:
                        while let Some(se) = popped.pop() {
                            current.children.push_back(se.element);
                        }

                        let se = StackElement::new(current, current_depth);
                        self.element_stack.push_back(se);
                    }
                }
                ParseState::State8 => {}
                ParseState::StateError => {}
            }

            self.state = new_state;
        }

        if self.element_stack.size() == 0 {
            return Err("no element parsed".to_string());
        }

        let stack_elem = self.element_stack.pop_back().unwrap();
        Ok(stack_elem.element)
    }

    fn get_next_token(&mut self) -> Option<XMLToken> {
        let bytes = self.input.as_bytes();
        let length = bytes.len();
        let begin_pos = self.position;

        if begin_pos >= length {
            return None;
        }

        while self.position < length {
            let ch = bytes[self.position];
            self.position += 1;

            if ch == BEGIN_TAG_TOKEN as u8 {
                if self.position > begin_pos + 1 {
                    // Found '<' after some text — emit the accumulated text first
                    self.position -= 1;
                    return Some(self.get_text_token(begin_pos, self.position - 1));
                } else {
                    // '<' is the very first char; check what follows
                    if self.position < length && bytes[self.position] == SPLASH_TOKEN as u8 {
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

        // End of input reached — emit any remaining text
        if self.position == 0 {
            return None;
        }
        Some(self.get_text_token(begin_pos, self.position - 1))
    }

    fn get_text_token(&self, mut from: usize, mut to: usize) -> XMLToken {
        let bytes = self.input.as_bytes();

        // Trim leading whitespace (spaces, newlines, tabs, CR)
        while from <= to && from < bytes.len() && is_xml_whitespace(bytes[from]) {
            from += 1;
        }
        // Trim trailing whitespace
        while to >= from && to < bytes.len() && is_xml_whitespace(bytes[to]) {
            if to == 0 {
                break;
            }
            to -= 1;
            // If we've gone past `from`, break
            if to < from {
                break;
            }
        }

        if to >= from && from < bytes.len() && to < bytes.len() && !is_xml_whitespace(bytes[from]) {
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
        use ParseState::*;
        use XMLTokenType::*;
        let table: [[ParseState; 4]; 8] = [
            // BeginOpenTag, BeginCloseTag, EndTag,     Text
            [State2,         StateError,    StateError, StateError], // State1
            [StateError,     StateError,    StateError, State3],     // State2
            [StateError,     StateError,    State4,     StateError], // State3
            [State2,         StateError,    StateError, State5],     // State4
            [StateError,     State6,        StateError, StateError], // State5
            [StateError,     StateError,    StateError, State7],     // State6
            [StateError,     StateError,    State8,     StateError], // State7
            [State2,         State6,        StateError, StateError], // State8
        ];
        let row = match state {
            State1 => 0,
            State2 => 1,
            State3 => 2,
            State4 => 3,
            State5 => 4,
            State6 => 5,
            State7 => 6,
            State8 => 7,
            StateError => return StateError,
        };
        let col = match token {
            BeginOpenTag => 0,
            BeginCloseTag => 1,
            EndTag => 2,
            Text => 3,
        };
        table[row][col]
    }

    fn release(&mut self) {
        self.element_stack.release();
        self.value_stack.release();
        self.tag_stack.release();
    }
}

fn is_xml_whitespace(b: u8) -> bool {
    b == b' ' || b == b'\n' || b == b'\r' || b == b'\t'
}

pub fn parse_xml_from_text(text: &str) -> Result<XMLElement, String> {
    let mut parser = XMLParser::new();
    parser.parse(text)
}
