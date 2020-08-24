use super::inkwell::types::AnyTypeEnum;

pub fn get_num_pointer_layers(val_type: AnyTypeEnum) -> u8 {
    let mut num_pointers = 0;
    let mut val_type = val_type;
    while val_type.is_pointer_type() {
        num_pointers += 1;
        val_type = val_type.into_pointer_type().get_element_type();
    }
    num_pointers
}