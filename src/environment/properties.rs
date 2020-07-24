use crate::ast::{Property, PropertyInformation};
use crate::environment::*;

impl Environment {
    pub fn add_property(
        &mut self,
        property: Property,
        identifier: &str,
        type_id: &str,
    ) {
        if let Some(type_info) = self.types.get_mut(type_id) {
            type_info
                .properties
                .insert(identifier.to_string(), PropertyInformation { property });
            type_info.ordered_properties.push(identifier.to_string());
        }
    }

    pub fn property(&self, identifier: &str, type_id: &str) -> Option<PropertyInformation> {
        self.types.get(type_id)
            .and_then(|type_info| type_info.properties.get(identifier))
            .cloned()
    }

    pub fn property_declarations(&self, type_id: &str) -> Vec<Property> {
        self.types.get(type_id).into_iter()
            .flat_map(|type_info| &type_info.properties)
            .map(|(_, v)| v.property.clone())
            .collect()
    }

    pub fn is_property_defined(&self, identifier: &str, type_id: &str) -> bool {
        self.property(identifier, type_id).is_some()
    }

    pub fn property_offset(&self, property: String, type_id: &str) -> u64 {
        self.types.get(type_id)
            .map(|root_type| root_type.ordered_properties
                .iter()
                .take_while(|&p| p != &property)
                .filter_map(|p| root_type.properties.get(p))
                .map(|info| self.type_size(&info.property.get_type()))
                .sum()).unwrap_or(0)
    }
}
