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
        // Drop children and clear strings to release resources
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

            // Skip empty TEXT tokens
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
                        if let Some(data) = token.data.clone() {
                            self.tag_stack.push_back(data);
                            self.depth += 1;
                        }
                    }
                }
                ParseState::State3 => {}
                ParseState::State4 => {
                    // Mirror C semantics: in STATE4, a value entry is always
                    // pushed when transitioning out (either with the text data,
                    // or with an empty placeholder when we're about to descend
                    // into a child tag). This keeps tag_stack and value_stack
                    // in lockstep so each element has a corresponding value.
                    match token.token_type {
                        XMLTokenType::Text => {
                            let data = token.data.clone().unwrap_or_default();
                            self.value_stack.push_back(data);
                        }
                        XMLTokenType::BeginOpenTag => {
                            self.value_stack.push_back(String::new());
                        }
                        _ => {}
                    }
                }
                ParseState::State5 => {}
                ParseState::State6 => {
                    if token.token_type == XMLTokenType::Text {
                        if let Some(data) = &token.data {
                            let top = self.tag_stack.top_back();
                            match top {
                                Some(t) => {
                                    if t != data {
                                        return Err(format!(
                                            "mismatched closing tag: expected {}, got {}",
                                            t, data
                                        ));
                                    }
                                }
                                None => {
                                    return Err("no opening tag for closing tag".to_string());
                                }
                            }
                        }
                    }
                }
                ParseState::State7 => {
                    if token.token_type == XMLTokenType::EndTag {
                        let current_tag = self
                            .tag_stack
                            .top_back()
                            .cloned()
                            .ok_or_else(|| "tag_stack empty".to_string())?;
                        let current_value = self
                            .value_stack
                            .top_back()
                            .cloned()
                            .ok_or_else(|| "value_stack empty".to_string())?;
                        let length = self.element_stack.size();
                        if self.depth == 0 {
                            return Err("depth underflow".to_string());
                        }
                        self.depth -= 1;

                        let mut current = XMLElement::new(current_tag, current_value);
                        let se_depth = self.depth;

                        // find children of current elem - take elements with higher depth
                        // Collect them in reverse-stack order, then push_front into children
                        // (the C code does push_front + pop_back, building children
                        //  in reverse order of pops which equals stack order).
                        for _ in 0..length {
                            let should_take = match self.element_stack.top_back() {
                                Some(elem) => elem.depth > se_depth,
                                None => false,
                            };
                            if !should_take {
                                break;
                            }
                            // pop_back the StackElement
                            if let Some(elem) = self.element_stack.pop_back() {
                                // push_front the contained XMLElement to current.children
                                current.children.push_front(elem.element);
                            }
                        }

                        // push to stack
                        let se = StackElement::new(current, se_depth);
                        self.element_stack.push_back(se);

                        self.tag_stack.pop_back();
                        self.value_stack.pop_back();
                    }
                }
                ParseState::State8 => {}
                ParseState::StateError => {}
            }

            self.state = next_state;
        }

        if self.element_stack.size() == 0 {
            return Err("no elements parsed".to_string());
        }
        let top = self
            .element_stack
            .pop_back()
            .ok_or_else(|| "element_stack empty".to_string())?;
        Ok(top.element)
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
                        // We've consumed text before the '<'; return it as text
                        self.position -= 1;
                        return Some(self.get_text_token(begin_pos, self.position - 1));
                    } else {
                        // self.position == begin_pos + 1, just advanced past '<'
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
        // trim leading and trailing spaces
        while from < bytes.len() && bytes[from] == b' ' {
            from += 1;
        }
        // C version reads parser->_input[to] == ' '. Note that to is treated as inclusive.
        // We may also need to be careful: if to underflows we should stop.
        loop {
            if to >= bytes.len() {
                break;
            }
            if bytes[to] != b' ' {
                break;
            }
            if to == 0 {
                // signal empty
                return XMLToken {
                    token_type: XMLTokenType::Text,
                    data: None,
                };
            }
            to -= 1;
        }

        if to >= from && to < bytes.len() {
            let str_size = to - from + 1;
            let slice = &bytes[from..from + str_size];
            let data = String::from_utf8_lossy(slice).to_string();
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
        // Mapping from C state_translate table:
        // STATE1: BEGIN_OPEN_TAG -> STATE2
        // STATE2: TEXT -> STATE3
        // STATE3: END_TAG -> STATE4
        // STATE4: BEGIN_OPEN_TAG -> STATE2; TEXT -> STATE5
        // STATE5: BEGIN_CLOSE_TAG -> STATE6
        // STATE6: TEXT -> STATE7
        // STATE7: END_TAG -> STATE8
        // STATE8: BEGIN_OPEN_TAG -> STATE2; BEGIN_CLOSE_TAG -> STATE6
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
        self.element_stack.release();
        self.value_stack.release();
        self.tag_stack.release();
        self.input.clear();
        self.position = 0;
        self.depth = 0;
        self.state = ParseState::State1;
    }
}

pub fn parse_xml_from_text(text: &str) -> Result<XMLElement, String> {
    let mut parser = XMLParser::new();
    let result = parser.parse(text);
    parser.release();
    result
}
