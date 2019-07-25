use crate::tokenizer::Token;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ValidationContext {
    // TODO use Value and combine ArrayValue and ObjectEntryValue
    ArrayStart,
    ArrayValue,
    ArrayComma,
    ArrayEnd,
    ObjectStart,
    ObjectEntryKey,
    ObjectEntryColon,
    ObjectEntryValue,
    ObjectEntryComma,
    ObjectEnd,
}

impl ValidationContext {
    #[inline]
    pub fn in_array(self) -> bool {
        use ValidationContext::*;
        match self {
            ArrayStart | ArrayValue | ArrayComma => true,
            _ => false,
        }
    }

    #[inline]
    pub fn in_object(self) -> bool {
        use ValidationContext::*;
        match self {
            ObjectStart | ObjectEntryKey | ObjectEntryColon | ObjectEntryValue
            | ObjectEntryComma => true,
            _ => false,
        }
    }

    #[inline]
    pub fn in_value(self) -> bool {
        use ValidationContext::*;
        match self {
            ArrayValue | ObjectEntryValue => true,
            _ => false,
        }
    }

    #[inline]
    pub fn before_value(self) -> bool {
        use ValidationContext::*;
        match self {
            ArrayValue | ObjectEntryValue => true,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ValidationState {
    Complete,
    // Incomplete(Option<JsonType>),
    // TODO emit object on closing a subobject?
    Incomplete,
    // TODO keep Ignored or just use Complete?
    Ignored,
    // Invalid,
}

#[derive(Debug)]
pub enum ValidationError {
    // TODO provide more context?
    // /// Encountered a token while inside a context which isn't acceptable
    // Invalid(ValidationContext),
    // /// Encountered a token which is only valid when inside a context
    // RequiresContext,
    /// Encountered a token which is invalid according to our current context.
    ///
    /// If the context is None, then it is a token which is only valid
    /// once inside a context (e.g. ArrayClose encountered before an ArrayStart)
    ///
    /// If context is Some(_), then the token was encountered in a position
    /// in which something else was expected. You can use the method `valid_sequents`
    /// to figure out what could have been valid contexts to follow.
    Invalid(Option<ValidationContext>),
    UnexpectedEndOfInput,
}

impl ValidationContext {
    // TODO self or &self?
    // const fn valid_sequents(&self) -> &'static [ValidationContext] {
    fn valid_sequents(self) -> &'static [ValidationContext] {
        use ValidationContext::*;
        match self {
            ArrayStart => &[ArrayValue, ArrayEnd],
            ArrayValue => &[ArrayComma, ArrayEnd],
            ArrayComma => &[ArrayValue],
            ArrayEnd => &[],
            ObjectStart => &[ObjectEntryKey, ObjectEnd],
            ObjectEntryKey => &[ObjectEntryColon],
            ObjectEntryColon => &[ObjectEntryValue],
            ObjectEntryValue => &[ObjectEntryComma, ObjectEnd],
            ObjectEntryComma => &[ObjectEntryKey],
            ObjectEnd => &[],
        }
    }

    // TODO self or &self?
    fn is_valid_sequent(self, next: Self) -> bool {
        use ValidationContext::*;
        match (self, next) {
            (ArrayStart, ArrayValue)
            | (ArrayStart, ArrayEnd)
            | (ArrayValue, ArrayComma)
            | (ArrayValue, ArrayEnd)
            | (ArrayComma, ArrayValue)
            | (ObjectStart, ObjectEntryKey)
            | (ObjectStart, ObjectEnd)
            | (ObjectEntryKey, ObjectEntryColon)
            | (ObjectEntryColon, ObjectEntryValue)
            | (ObjectEntryValue, ObjectEntryComma)
            | (ObjectEntryValue, ObjectEnd)
            | (ObjectEntryComma, ObjectEntryKey) => true,
            _ => false,
        }
        // // match (self, next) {
        // for c in self.valid_sequents() {
        //     if c == next {
        //         return true;
        //     }
        // }
        // false
    }
}

#[derive(Default)]
pub struct Validator {
    // TODO benchmark version with only Vec + .last()
    current_context: Option<ValidationContext>,
    context_stack: Vec<ValidationContext>,
}

// TODO implement subcontext?

// trait ContextStack {
//     fn current(&self) -> &Option<ValidationContext>;
//     fn current_context(&mut self) -> &Option<ValidationContext>;
//     fn pop_until(&mut self, target: ValidationContext) -> bool;
//     fn pop(&mut self) -> Option<ValidationContext>;
//     fn push(&mut self);
// }

impl Validator {
    #[inline]
    pub fn new() -> Self {
        Validator {
            current_context: None,
            context_stack: vec![],
        }
    }

    #[inline]
    fn pop_until(&mut self, target: ValidationContext) -> bool {
        if let Some(idx) = self.context_stack.iter().rposition(|x| *x == target) {
            self.context_stack.truncate(idx);
            true
        } else {
            false
        }
    }

    pub fn current_context(&self) -> Option<ValidationContext> {
        self.current_context
    }

    // This method is called after we have finished closing something. After
    // closing something, it can either be a value in a larger context, or the
    // end of the input. It then represents the closed object as a subelement
    // in the larger context depending on what the context was.
    #[inline]
    fn check_completion(&mut self) -> Result<ValidationState, ValidationError> {
        use ValidationContext::*;
        use ValidationState::*;
        if let Some(end_context) = self.context_stack.last() {
            match end_context {
                ObjectEntryColon => {
                    self.current_context = Some(ObjectEntryValue);
                }
                ArrayStart | ArrayComma => {
                    self.current_context = Some(ArrayValue);
                }
                _ => unreachable!(), // TODO or is this panic!()?
            }
            Ok(Incomplete)
        } else {
            self.current_context = None;
            Ok(Complete)
        }
    }

    #[inline]
    fn transition_incomplete(
        &mut self,
        current: ValidationContext,
        next: ValidationContext,
    ) -> Result<ValidationState, ValidationError> {
        self.current_context = Some(next);
        self.context_stack.push(current);
        Ok(ValidationState::Incomplete)
    }

    pub fn finish(&mut self) -> Result<ValidationState, ValidationError> {
        if self.current_context.is_none() && self.context_stack.is_empty() {
            Ok(ValidationState::Complete)
        } else {
            Err(ValidationError::UnexpectedEndOfInput)
        }
    }

    pub fn reset(&mut self) {
        self.current_context = None;
        self.context_stack.clear();
    }

    #[inline]
    pub fn process_token(&mut self, token: &Token<'_>) -> Result<ValidationState, ValidationError> {
        use Token::*;
        use ValidationContext::*;
        use ValidationState::*;

        if token.is_whitespace() {
            return Ok(Ignored);
        }

        if let Some(context) = self.current_context {
            match context {
                ArrayStart => match token {
                    True | False | Null | Token::Number(_) | Token::String(_) => {
                        self.transition_incomplete(context, ArrayValue)
                    }
                    ObjectOpen => self.transition_incomplete(context, ObjectStart),
                    ArrayOpen => self.transition_incomplete(context, ArrayStart),
                    ArrayClose => self.check_completion(),
                    _ => Err(ValidationError::Invalid(self.current_context)),
                },
                ArrayValue => match token {
                    Comma => self.transition_incomplete(context, ArrayComma),
                    ArrayClose => {
                        if self.pop_until(ArrayStart) {
                            self.check_completion()
                        } else {
                            unreachable!();
                            // panic!("Couldn't find opening delimiter {:?}", ArrayStart);
                        }
                    }
                    _ => Err(ValidationError::Invalid(self.current_context)),
                },
                ArrayComma => match token {
                    True | False | Null | Token::Number(_) | Token::String(_) => {
                        self.transition_incomplete(context, ArrayValue)
                    }
                    ObjectOpen => self.transition_incomplete(context, ObjectStart),
                    ArrayOpen => self.transition_incomplete(context, ArrayStart),
                    _ => Err(ValidationError::Invalid(self.current_context)),
                },
                ObjectEnd | ArrayEnd => unreachable!(),
                ObjectStart => match token {
                    Token::String(_) => self.transition_incomplete(context, ObjectEntryKey),
                    ObjectClose => self.check_completion(),
                    _ => Err(ValidationError::Invalid(self.current_context)),
                },
                ObjectEntryKey => match token {
                    Colon => self.transition_incomplete(context, ObjectEntryColon),
                    _ => Err(ValidationError::Invalid(self.current_context)),
                },
                ObjectEntryColon => match token {
                    True | False | Null | Token::Number(_) | Token::String(_) => {
                        self.transition_incomplete(context, ObjectEntryValue)
                    }
                    ObjectOpen => self.transition_incomplete(context, ObjectStart),
                    ArrayOpen => self.transition_incomplete(context, ArrayStart),
                    _ => Err(ValidationError::Invalid(self.current_context)),
                },
                ObjectEntryValue => match token {
                    Comma => self.transition_incomplete(context, ObjectEntryComma),
                    ObjectClose => {
                        if self.pop_until(ObjectStart) {
                            self.check_completion()
                        } else {
                            unreachable!();
                            // panic!("Couldn't find opening delimiter {:?}", ObjectStart);
                        }
                    }
                    _ => Err(ValidationError::Invalid(self.current_context)),
                },
                ObjectEntryComma => match token {
                    Token::String(_) => self.transition_incomplete(context, ObjectEntryKey),
                    _ => Err(ValidationError::Invalid(self.current_context)),
                },
            }
        } else {
            match token {
                True | False | Token::Null | Token::Number(_) | Token::String(_) => Ok(Complete),
                Token::ArrayOpen => {
                    self.current_context = Some(ArrayStart);
                    Ok(Incomplete)
                }
                Token::ObjectOpen => {
                    self.current_context = Some(ObjectStart);
                    Ok(Incomplete)
                }
                // Token::Spaces(_) | Token::Whitespace(_) => unreachable!(),
                _ => Err(ValidationError::Invalid(None)),
            }
        }
    }

    // TODO this could be more efficient since it can avoid allocations to the context_stack
    // pub fn process_iterator(&mut self, token: Token<'_>) -> Result<ValidationState, ValidationError> {
    //     if let Some(context) = self.current_context {
    //     }
    // }
}
