use crate::ast::ContractBehaviourMember;

pub(crate) mod type_states;
pub mod unique;

pub fn is_init_declaration(member: &ContractBehaviourMember) -> bool {
    if let ContractBehaviourMember::SpecialDeclaration(special) = member {
        special.head.special_token.eq("init")
    } else {
        false
    }
}
