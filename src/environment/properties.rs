use crate::environment::*;

impl Environment {
    pub fn add_property(
        &mut self,
        property: Property,
        identifier: &TypeIdentifier,
        t: &TypeIdentifier,
    ) {
        let type_info = &self.types.get_mut(t);
        if type_info.is_some() {
            self.types
                .get_mut(t)
                .unwrap()
                .properties
                .insert(identifier.to_string(), PropertyInformation { property });
            self.types
                .get_mut(t)
                .unwrap()
                .ordered_properties
                .push(identifier.to_string());
        }
    }

    pub fn property(&self, identifier: String, t: &TypeIdentifier) -> Option<PropertyInformation> {
        let type_info = &self.types.get(t);
        if type_info.is_some() {
            let properties = &self
                .types
                .get(t)
                .unwrap()
                .properties
                .get(identifier.as_str());
            if properties.is_some() {
                let property = properties.unwrap().clone();
                return Some(property);
            }
        }
        None
    }

    pub fn property_declarations(&self, t: &TypeIdentifier) -> Vec<Property> {
        let type_info = &self.types.get(t);
        if type_info.is_some() {
            let properties: Vec<Property> = self
                .types
                .get(t)
                .unwrap()
                .properties
                .clone()
                .into_iter()
                .map(|(_, v)| v.property)
                .collect();
            return properties;
        }
        return vec![];
    }

    pub fn is_property_defined(&self, identifier: String, t: &TypeIdentifier) -> bool {
        self.property(identifier, t).is_some()
    }

    pub fn is_property_constant(&self, identifier: String, t: &TypeIdentifier) -> bool {
        if self.property(identifier.clone(), t).is_some() {
            return self.property(identifier, t).unwrap().is_constant();
        }
        false
    }

    pub fn property_offset(&self, property: String, t: &TypeIdentifier) -> u64 {
        let mut offset_map: HashMap<String, u64> = HashMap::new();
        let mut offset: u64 = 0;

        let root_type = self.types.get(t);
        if root_type.is_some() {
            let root_type = root_type.unwrap();
            let ordered_properties = root_type.ordered_properties.clone();
            let ordered_properties: Vec<String> = ordered_properties
                .into_iter()
                .take_while(|p| p.to_string() != property)
                .collect();
            for p in ordered_properties {
                offset_map.insert(p.clone(), offset);
                let property_type = root_type.properties.get(&p);
                let property_type = property_type.unwrap();
                let property_size = self.type_size(property_type.property.get_type());

                offset = offset + property_size;
            }
            offset
        } else {
            offset
        }
    }
}