use lazy_static::lazy_static;
use std::collections::HashMap;
use crate::COMPONENT_TO_FUNCTION_MAP;

const FUNCTION_NAME_SIZE: usize = 30;

fn parse_spell() {}

fn parse_component() {

}

fn pad_component_name(component_name: &str) -> [Option<char>; FUNCTION_NAME_SIZE] {
    let mut padded_name = [None; FUNCTION_NAME_SIZE];
    for (index, character) in component_name.chars().take(FUNCTION_NAME_SIZE).enumerate() {
        padded_name[index] = Some(character);
    }
    padded_name
}

fn decode_component_name(padded_name: &[Option<char>; FUNCTION_NAME_SIZE]) -> String {
    padded_name.iter()
    .filter_map(|&character| character)
    .collect()
}

lazy_static! {
    static ref COMPONENT_TO_NUM_MAP: HashMap<[Option<char>; FUNCTION_NAME_SIZE], u64> = {
        let mut component_map = HashMap::new();
        component_map.insert(pad_component_name("give_velocity"), 0);
        component_map
    };
}


fn get_component_num(component_name: &str) -> Option<&u64> {
    COMPONENT_TO_NUM_MAP.get(&pad_component_name(component_name))
}

fn parse_component_string(component_call: &str) -> Result<(String, Vec<Parameter>), &'static str> {
    if component_call.chars().last() != Some(')') {
        return Err("Invalid component: Must end with close bracket");
    }

    let mut component_name = String::new();
    let mut character_count = 0;
    let mut found_opening_bracket = false;

    // Looping through component_call to get component_name
    for character in component_call.chars() {
        if character == '(' {
            if character == ' '{
                continue;
            } else if character == ',' {
                return Err("Invalid component: Must begin with letters")
            }
            found_opening_bracket = true;
            break;
            // Checking if character is alphabetic if not an open bracket.
        } else if !character.is_alphabetic() {
            return Err("Invalid component: Name must be made up of letters")
        }

        character_count += 1;
        component_name.push(character);
    }

    // There needs to be an opening bracket, if there is none, returns error
    if found_opening_bracket == false {
        return Err("Invalid component: Must have opening bracket")
    }

    // This line gets the parameters as a string and puts it into the variable parameters_string
    if let Some(parameters_string) = component_call.get(character_count + 1..component_call.len() - 1) {
        let parameters = collect_parameters(parameters_string, &component_name)?;
        return Ok((component_name, parameters))
    } else {
        return Err("Invalid component: Parameters not valid")
    }
}

enum Parameter {
    Integer(u64),
    Float(f64),
    Boolean(bool)
}


fn collect_parameters(parameters_string: &str, component_name: &str) -> Result<Vec<Parameter>, &'static str> {
    let mut parameter = String::new();
    let mut parameters: Vec<Parameter> = vec![];

    // ToDo: Use COMPONENT_TO_FUNCTION_MAP to find expected parameter type and attempt to convert it

    /*
    if let Some((_, encoded_types)) = COMPONENT_TO_FUNCTION_MAP.get(COMPONENT_TO_NUM_MAP.get(get_component_num(component_name).expect("Expected component"))) {
        let encoded_types: &[u64] = encoded_types;
        for &parameter_type in encoded_types {

        }
    } else {
        panic!("Expected component mapping")
    }
    */
    for character in parameters_string.chars() {
        if character == ',' {
            if parameter.is_empty() {
                return Err("Invalid component: Must have value before bracket")
            }
            // Adding parameter to parameters vector
            parameters.push(parse_parameter(&parameter, component_name));

            // Clear parameter string so next one can be recorded
            parameter.clear()

        } else {
            parameter.push(character)
        }
    }

    // Adding last parameter
    if !parameter.is_empty() {
        parameters.push(parse_parameter(&parameter, component_name));
    }

    return Ok(parameters)
}

fn parse_parameter(parameters_string: &str, component_name: &str) -> Parameter {

    return Parameter::Integer(0)

}
