use fractic_server_error::{
    ServerError, ServerErrorBehaviour, ServerErrorContext, ServerErrorTag, ServerErrorTrait,
};

#[derive(Debug)]
struct TaggedError {
    context: String,
    message: String,
    debug: Option<String>,
    behaviour: ServerErrorBehaviour,
    tag: ServerErrorTag,
}

impl TaggedError {
    #[track_caller]
    fn wrap(line_id: u64, error: ServerError) -> ServerError {
        Box::new(Self {
            context: ServerErrorContext::Partial.capture(),
            message: format!("@ Line {}: {}", line_id, error.message()),
            debug: error.debug().cloned(),
            behaviour: error.behaviour(),
            tag: error.tag(),
        })
    }
}

impl ServerErrorTrait for TaggedError {
    fn behaviour(&self) -> ServerErrorBehaviour {
        self.behaviour.clone()
    }

    fn tag(&self) -> ServerErrorTag {
        self.tag.clone()
    }

    fn context(&self) -> &String {
        &self.context
    }

    fn message(&self) -> &String {
        &self.message
    }

    fn debug(&self) -> Option<&String> {
        self.debug.as_ref()
    }
}

#[track_caller]
pub(crate) fn with_line_id(line_id: u64, error: ServerError) -> ServerError {
    TaggedError::wrap(line_id, error)
}
