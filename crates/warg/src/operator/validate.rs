use super::model::Entry;

pub struct State {}

pub enum ValidationError {}

pub fn validate(state: State, entry: Entry) -> Result<State, ValidationError> {
    todo!()
}
