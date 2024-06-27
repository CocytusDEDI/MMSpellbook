fn get_component(component_call: &str) -> Result<(String, Vec<Parameter>), &'static str> {
    if component_call.chars().last() != Some(')') {
        return Err("Invalid component: Must end with close bracket");
    }

    let mut component_name = String::new();
    let mut character_count = 0;
    let mut found_opening_bracket = false;

    // Looping through component_call to get component_name
    for character in component_call.chars() {
        if character == '(' {
            if character == ' ' {
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
        return Ok((component_name, collect_parameters(parameters_string)?))
    } else {
        return Err("Invalid component: Parameters not valid")
    }
}

enum Parameter {
    Integer(i32),
    Float(f64),
    String(String),
    Boolean(bool),
}

// Turns a string in the form "parameter, parameter, ..." into a vector of type Parameter
fn collect_parameters(parameters_string: &str) -> Result<Vec<Parameter>, &'static str> {
    let mut parameter = String::new();
    let mut parameters: Vec<Parameter> = vec![];

    for character in parameters_string.chars() {
        if character == ',' {
            if parameter.is_empty() {
                return Err("Invalid component: Must have value before bracket")
            }
            // Adding parameter to parameters vector
            parameters.push(parse_parameter(&parameter)?);

            // Clear parameter string so next one can be recorded
            parameter.clear()

        } else {
            parameter.push(character)
        }
    }

    // Adding last parameter
    if !parameter.is_empty() {
        parameters.push(parse_parameter(&parameter)?);
    }

    return Ok(parameters)
}

// Turns a string into a Parameter datatype
fn parse_parameter(parameter: &str) -> Result<Parameter, &'static str> {
    // Attempts to convert string into datatypes until one is successful
    
    if let Ok(value) = parameter.parse::<i32>() {
        return Ok(Parameter::Integer(value))
    }

    if let Ok(value) = parameter.parse::<f64>() {
        return Ok(Parameter::Float(value))
    }

    if let Ok(value) = parameter.parse::<bool>() {
        return Ok(Parameter::Boolean(value))
    }

    if let Ok(value) = parameter.parse::<String>() {
        // Checking if string has not allowed characters (any non alphabetic character)
        if !value.chars().all(char::is_alphabetic) {
            return Err("Invalid parameter: Strings must only contain letters")
        }
        return Ok(Parameter::String(value))
    }

    return Err("Invalid parameter")
}
