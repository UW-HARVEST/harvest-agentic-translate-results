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
        // Drop children explicitly, mirroring C-style release
        self.element.children.release();
        self.element.tag_name.clear();
        self.element.value.clear();
        self.depth = 0;
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

            // Skip empty TEXT tokens (mirror C behavior)
            if token.token_type == XMLTokenType::Text && token.data.is_none() {
                continue;
            }

            let new_state = Self::translate(self.state, &token.token_type);

            if new_state != ParseState::StateError {
                match self.state {
                    ParseState::State1 => {}
                    ParseState::State2 => {
                        if token.token_type == XMLTokenType::Text {
                            if let Some(ref d) = token.data {
                                self.tag_stack.push_back(d.clone());
                                self.depth += 1;
                            }
                        }
                    }
                    ParseState::State3 => {}
                    ParseState::State4 => {
                        // STATE4 transitions on TEXT to STATE5 — push value
                        // Or on BEGIN_OPEN_TAG to STATE2 — push empty value
                        // C code unconditionally pushes token->data; for BEGIN_OPEN_TAG
                        // there is no data so push empty string.
                        let val = token.data.clone().unwrap_or_default();
                        self.value_stack.push_back(val);
                    }
                    ParseState::State5 => {}
                    ParseState::State6 => {
                        if token.token_type == XMLTokenType::Text {
                            if let Some(ref d) = token.data {
                                if let Some(top) = self.tag_stack.top_back() {
                                    if d != top {
                                        return Err(format!(
                                            "mismatched closing tag: expected '{}' got '{}'",
                                            top, d
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    ParseState::State7 => {
                        if token.token_type == XMLTokenType::EndTag {
                            // Build a current element from top of tag_stack and value_stack
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
                            if self.depth > 0 {
                                self.depth -= 1;
                            }

                            let mut current = XMLElement::new(current_tag, current_value);
                            let se_depth = self.depth;

                            // Find children of current elem
                            // Pop StackElements with depth > se_depth and push their
                            // elements to the front of current.children (preserve order).
                            for _ in 0..length {
                                let should_take = match self.element_stack.top_back() {
                                    Some(elem) => elem.depth > se_depth,
                                    None => false,
                                };
                                if !should_take {
                                    break;
                                }
                                if let Some(elem) = self.element_stack.pop_back() {
                                    current.children.push_front(elem.element);
                                }
                            }

                            // Push current StackElement to the element_stack
                            let se = StackElement::new(current, se_depth);
                            self.element_stack.push_back(se);

                            self.tag_stack.pop_back();
                            self.value_stack.pop_back();
                        }
                    }
                    ParseState::State8 => {}
                    ParseState::StateError => {}
                }
            }

            if new_state == ParseState::StateError {
                self.release();
                return Err("error while parsing".to_string());
            }
            self.state = new_state;
        }

        // The top StackElement holds the root XMLElement
        let root_se = self.element_stack.pop_back();
        self.release();
        match root_se {
            Some(se) => Ok(se.element),
            None => Err("no root element parsed".to_string()),
        }
    }

    fn get_next_token(&mut self) -> Option<XMLToken> {
        let bytes: Vec<char> = self.input.chars().collect();
        let length = bytes.len();
        let begin_pos = self.position;

        if begin_pos >= length {
            return None;
        }

        while self.position < length {
            let ch = bytes[self.position];
            self.position += 1;

            match ch {
                BEGIN_TAG_TOKEN => {
                    if self.position > begin_pos + 1 {
                        // We've consumed some text before this '<'
                        self.position -= 1;
                        return Some(self.get_text_token(begin_pos, self.position - 1));
                    } else {
                        // self.position == begin_pos + 1, look at next char
                        if self.position < length && bytes[self.position] == SPLASH_TOKEN {
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

        Some(self.get_text_token(begin_pos, self.position.saturating_sub(1)))
    }

    fn get_text_token(&self, mut from: usize, mut to: usize) -> XMLToken {
        let chars: Vec<char> = self.input.chars().collect();
        let length = chars.len();

        // Trim leading spaces
        while from < length && chars[from] == ' ' {
            from += 1;
        }
        // Trim trailing spaces (`to` must remain valid; treat positions outside as non-space)
        while to < length && to >= from && chars[to] == ' ' {
            if to == 0 {
                break;
            }
            to -= 1;
        }

        let data = if to >= from && from < length && to < length {
            let s: String = chars[from..=to].iter().collect();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        } else {
            None
        };

        XMLToken {
            token_type: XMLTokenType::Text,
            data,
        }
    }

    fn translate(state: ParseState, token: &XMLTokenType) -> ParseState {
        // Mirror state_translate[8][4] from C
        // Columns: BeginOpenTag, BeginCloseTag, EndTag, Text
        use ParseState::*;
        use XMLTokenType::*;
        let row = match state {
            State1 => [State2, StateError, StateError, StateError],
            State2 => [StateError, StateError, StateError, State3],
            State3 => [StateError, StateError, State4, StateError],
            State4 => [State2, StateError, StateError, State5],
            State5 => [StateError, State6, StateError, StateError],
            State6 => [StateError, StateError, StateError, State7],
            State7 => [StateError, StateError, State8, StateError],
            State8 => [State2, State6, StateError, StateError],
            StateError => [StateError, StateError, StateError, StateError],
        };
        let col = match token {
            BeginOpenTag => 0,
            BeginCloseTag => 1,
            EndTag => 2,
            Text => 3,
        };
        row[col]
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
