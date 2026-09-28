use es_fluent::EsFluent;

#[derive(Clone, Copy, Debug, Eq, EsFluent, PartialEq)]
pub enum FormAction {
    Submit,
    Reset,
}
