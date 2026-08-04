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
        self.depth = 0;
        self.element.children.release();
        self.element.tag_name.clear();
        self.element.value.clear();
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
        self.input.clear();
        self.input.push_str(text);
        self.position = 0;
        self.depth = 0;
        self.state = ParseState::State1;
        self.tag_stack.release();
        self.value_stack.release();
        self.element_stack.release();
        self.tag_stack = Vector::new(8);
        self.value_stack = Vector::new(8);
        self.element_stack = Vector::new(8);

        while let Some(token) = self.get_next_token() {
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
                    if let Some(tag) = token.data {
                        self.tag_stack.push_back(tag);
                        self.depth += 1;
                    }
                }
                ParseState::State3 => {}
                ParseState::State4 => {
                    if let Some(value) = token.data {
                        self.value_stack.push_back(value);
                    }
                }
                ParseState::State5 => {}
                ParseState::State6 => {
                    if let Some(close_tag) = token.data.as_deref() {
                        match self.tag_stack.top_back() {
                            Some(current_tag) if current_tag == close_tag => {}
                            _ => return Err("error while parsing".to_string()),
                        }
                    }
                }
                ParseState::State7 => {
                    if token.token_type == XMLTokenType::EndTag {
                        let current_tag = self
                            .tag_stack
                            .top_back()
                            .cloned()
                            .ok_or_else(|| "error while parsing".to_string())?;
                        let current_value = self
                            .value_stack
                            .top_back()
                            .cloned()
                            .unwrap_or_default();
                        self.depth = self.depth.saturating_sub(1);
                        let current_depth = self.depth;

                        let mut current = XMLElement::new(current_tag, current_value);
                        while matches!(
                            self.element_stack.data.last(),
                            Some(StackElement { depth, .. }) if *depth > current_depth
                        ) {
                            let child = self
                                .element_stack
                                .pop_back()
                                .ok_or_else(|| "error while parsing".to_string())?;
                            current.children.push_front(child.element);
                        }

                        self.element_stack
                            .push_back(StackElement::new(current, current_depth));
                        self.tag_stack.pop_back();
                        self.value_stack.pop_back();
                    }
                }
                ParseState::State8 => {}
                ParseState::StateError => return Err("error while parsing".to_string()),
            }

            self.state = next_state;
        }

        let root = self
            .element_stack
            .pop_back()
            .ok_or_else(|| "error while parsing".to_string())?;
        self.release();
        Ok(root.element)
    }

    fn get_next_token(&mut self) -> Option<XMLToken> {
        let begin_pos = self.position;
        if begin_pos >= self.input.len() {
            return None;
        }

        while self.position < self.input.len() {
            let ch = self.input[self.position..].chars().next()?;
            self.position += ch.len_utf8();

            match ch {
                BEGIN_TAG_TOKEN => {
                    if self.position > begin_pos + 1 {
                        self.position -= ch.len_utf8();
                        return Some(self.get_text_token(begin_pos, self.position - 1));
                    }

                    let next_char = self.input[self.position..].chars().next()?;
                    if next_char == SPLASH_TOKEN {
                        self.position += next_char.len_utf8();
                        return Some(XMLToken {
                            token_type: XMLTokenType::BeginCloseTag,
                            data: None,
                        });
                    }

                    return Some(XMLToken {
                        token_type: XMLTokenType::BeginOpenTag,
                        data: None,
                    });
                }
                END_TAG_TOKEN => {
                    if self.position > begin_pos + 1 {
                        self.position -= ch.len_utf8();
                        return Some(self.get_text_token(begin_pos, self.position - 1));
                    }

                    return Some(XMLToken {
                        token_type: XMLTokenType::EndTag,
                        data: None,
                    });
                }
                _ => {}
            }
        }

        Some(self.get_text_token(begin_pos, self.position - 1))
    }

    fn get_text_token(&self, mut from: usize, mut to: usize) -> XMLToken {
        let bytes = self.input.as_bytes();
        while from <= to && bytes[from].is_ascii_whitespace() {
            from += 1;
        }
        while from <= to && bytes[to].is_ascii_whitespace() {
            to = to.saturating_sub(1);
        }

        let data = if from <= to {
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
        self.input.clear();
        self.position = 0;
        self.depth = 0;
        self.state = ParseState::State1;
    }
}

pub fn parse_xml_from_text(text: &str) -> Result<XMLElement, String> {
    XMLParser::new().parse(text)
}
