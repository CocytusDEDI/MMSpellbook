use lazy_static::lazy_static;
use std::collections::HashMap;
use crate::{ReturnType, COMPONENT_TO_FUNCTION_MAP, Spell, boolean_logic, rpn_operations, codes::componentcodes::*, codes::opcodes::*, codes::attributecodes::*};

const FUNCTION_NAME_SIZE: usize = 25;

const ON_READY_NAME: &'static str = "when_created";
const PROCESS_NAME: &'static str = "repeat";
const META_DATA_NAME: &'static str = "about";

fn pad_component_name(component_name: &str) -> [Option<char>; FUNCTION_NAME_SIZE] {
    let mut padded_name = [None; FUNCTION_NAME_SIZE];
    for (index, character) in component_name.chars().take(FUNCTION_NAME_SIZE).enumerate() {
        padded_name[index] = Some(character);
    }
    padded_name
}

lazy_static! {
    static ref COMPONENT_TO_NUM_MAP: HashMap<[Option<char>; FUNCTION_NAME_SIZE], u64> = {
        let mut component_map = HashMap::new();

        // Utility:
        component_map.insert(pad_component_name("give_velocity"), GIVE_VELOCITY);
        component_map.insert(pad_component_name("take_form"), TAKE_FORM);
        component_map.insert(pad_component_name("undo_form"), UNDO_FORM);
        component_map.insert(pad_component_name("recharge_to"), RECHARGE_TO);
        component_map.insert(pad_component_name("anchor"), ANCHOR);
        component_map.insert(pad_component_name("undo_anchor"), UNDO_ANCHOR);

        // Logic:
        component_map.insert(pad_component_name("moving"), MOVING);
        component_map.insert(pad_component_name("get_time"), GET_TIME);

        // Power:
        component_map.insert(pad_component_name("set_damage"), SET_DAMAGE);

        component_map
    };
}

pub fn get_component_num(component_name: &str) -> Option<u64> {
    COMPONENT_TO_NUM_MAP.get(&pad_component_name(component_name)).cloned()
}

pub fn parse_spell(spell_code: &str) -> Result<Vec<u64>, &'static str> {
    let mut instructions: Vec<u64> = vec![];
    let mut in_section = None;
    let mut expected_closing_brackets: usize = 0;
    let trimmed_spell_code = spell_code.trim();
    for line in trimmed_spell_code.lines() {
        let trimmed_line = line.trim();
        if trimmed_line.ends_with(":") && trimmed_line.chars().take(trimmed_line.len() - 1).all(|character| character.is_alphanumeric() || character == '_' || character == ' ') {
            match trimmed_line.trim_end_matches(':').split_whitespace().collect::<Vec<&str>>()[..] {
                [ON_READY_NAME] => instructions.push(READY_SECTION),
                [PROCESS_NAME] => {
                    instructions.extend(vec![PROCESS_SECTION, NUMBER_LITERAL, f64::to_bits(1.0)]);
                },
                [PROCESS_NAME, "every", num] => {
                    instructions.extend(vec![PROCESS_SECTION, NUMBER_LITERAL, num.parse::<u64>().map(|num| f64::to_bits(num as f64)).map_err(|_| "Invalid value found after keyword \"every\"")?]);
                },
                [META_DATA_NAME] => instructions.push(METADATA_SECTION),
                _ => return Err("Invalid section name")
            };
            in_section = instructions.last().copied();
        } else {
            if let None = in_section {
                return Err("Must begin with section statement");
            }

            if Some(METADATA_SECTION) == in_section {
                if trimmed_line.contains('=') { // Indicates an assignment of metadata
                    instructions.extend(parse_about_line(trimmed_line)?);
                    continue
                } else if trimmed_line == "" {
                    continue
                } else {
                    return Err("Expected attribute in about section")
                }
            }
            
            // If in section, parse code
            if trimmed_line.ends_with(")") { // Checking to see if component
                instructions.extend(parse_component(trimmed_line)?);
            } else if trimmed_line.starts_with("if ") && trimmed_line.ends_with("{") { // Checking for if statement
                instructions.push(IF); // Indicates if statement
                instructions.extend(parse_logic(&trimmed_line[3..trimmed_line.len() - 1])?);
                instructions.push(0); // Indicates end of scope for logic
                expected_closing_brackets += 1;
            } else if expected_closing_brackets > 0 && trimmed_line == "}" {
                instructions.push(END_OF_SCOPE);
                expected_closing_brackets -= 1;
            } else if trimmed_line == "" {
                continue
            } else {
                return Err("Not acceptable statement")
            }
        }
    }
    if expected_closing_brackets == 0 {
        return Ok(instructions)
    } else {
        return Err("Expected closing bracket(s)")
    }
}

fn get_precedence(operator: &str) -> u64 {
    match operator {
        "(" | ")" => 0,
        "and" | "or" | "xor" => 1,
        ">" | "<" | "=" | "==" => 2,
        "+" | "-" => 3,
        "*" | "/" => 4,
        "^" => 5,
        "not" => 6,
        _ => panic!("Not valid operator")
    }
}

#[derive(PartialEq, Eq)]
enum Direction {
    Left,
    Right
}

fn get_associative_direction(operator: &str) -> Direction {
    match operator {
        "and" | "or" | "xor" | "+" | "-" | "*" | "/" | "^" | "=" | "==" | ">" | "<" => Direction::Left,
        "not" => Direction::Right,
        _ => panic!("Not valid operator")
    }
}

enum Token {
    Opcode(String),
    Number(String),
    Boolean(String),
    Component(String),
    OpenBracket,
    CloseBracket
}

fn tokenise(conditions: &str) -> Result<Vec<Token>, &'static str> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut characters = conditions.chars().peekable();
    let mut last_token_was_value = false;
    let mut close_bracket = false;
    let mut close_extra_bracket: usize = 0;

    while let Some(&character) = characters.peek() {
        match character {
            ' ' => {
                characters.next();
            },
            '(' => {
                tokens.push(Token::OpenBracket);
                characters.next();
            },
            ')' => {
                tokens.push(Token::CloseBracket);
                if close_extra_bracket > 0 {
                    tokens.push(Token::CloseBracket);
                    close_extra_bracket -= 1;
                }
                characters.next();
            },
            '+' | '*' | '/' | '^' | '=' | '>' | '<' => {
                let mut opcode = String::new();
                opcode.push(characters.next().unwrap());
                if let Some('=') = characters.peek() {
                    opcode.push(characters.next().unwrap());
                }
                tokens.push(Token::Opcode(opcode));
            },
            '-' => {
                // Standardises all minus signs to be a subtraction
                // If next character is - or +, collapse into one character
                // If last token was value, push plus
                // Then, push (0 -
                // If number or component make close_bracket = true
                    // In number, add closing bracket if true
                    // In component, add extra closing bracket if true
                // If open bracket, close_extra_bracket += 1
                    // In closing bracket, add another closing bracket and decrease by one if greater than zero

                // So ---(5 - (3 + 2)) = (0 - (5 + (0 - (3 + 2))))

                characters.next();
                let mut minus_count: usize = 1;
                while let Some(&next_character) = characters.peek() {
                    if next_character == '-' {
                        minus_count += 1;
                        characters.next();
                    } else if next_character == '+' {
                        characters.next();
                    } else {
                        break;
                    }
                }

                if minus_count % 2 == 0 { // if overall positive, move to next character in loop
                    continue
                }

                if last_token_was_value {
                    tokens.push(Token::Opcode("+".to_string()))
                }

                tokens.push(Token::OpenBracket);
                tokens.push(Token::Number("0".to_string()));
                tokens.push(Token::Opcode("-".to_string()));

                let mut at_least_one_loop = false;
                while let Some(&next_character) = characters.peek() {
                    if next_character.is_alphanumeric() { // If is number or character as if character we assume it's a component
                        close_bracket = true;
                        at_least_one_loop = true;
                        break;
                    } else if next_character == '(' {
                        close_extra_bracket += 1;
                        at_least_one_loop = true;
                        break;
                    } else if next_character == ' '{
                        characters.next();
                    } else {
                        return Err("Expected valid character after minus sign")
                    }
                }
                if !at_least_one_loop {
                    return Err("Expected character after minus sign")
                }
            },
            'a'..='z' | 'A'..='Z' | '_' => {
                let mut opcode = String::new();
                while let Some(&letter) = characters.peek() {
                    if letter.is_alphanumeric() || letter == '_' {
                        opcode.push(letter);
                        characters.next();
                    } else {
                        break;
                    }
                }
                if let Some('(') = characters.peek() { // Is component
                    opcode.push(characters.next().unwrap()); // Push OpenBracket
                    let mut expected_closing_brackets = 1;
                    loop {
                        if let Some(')') = characters.peek() {
                            // Push closing bracket
                            opcode.push(characters.next().unwrap());
                            expected_closing_brackets -= 1;
                        } else if let Some('(') = characters.peek() {
                            opcode.push(characters.next().unwrap());
                            expected_closing_brackets += 1;
                        } else {
                            // Push parameter characters
                            opcode.push(characters.next().ok_or("Expected closing bracket for component")?);
                        }
                        if expected_closing_brackets == 0 {
                            tokens.push(Token::Component(opcode));
                            break;
                        }
                    }
                    last_token_was_value = true;
                    if close_bracket {
                        tokens.push(Token::CloseBracket);
                        close_bracket = false;
                    }
                } else if opcode == "true" || opcode == "false" {
                    tokens.push(Token::Boolean(opcode));
                } else {
                    tokens.push(Token::Opcode(opcode));
                }
            },
            '0'..='9' => {
                let mut decimal_point_found = false;
                let mut number = String::new();
                while let Some(&number_character) = characters.peek() {
                    if number_character.is_numeric() {
                        number.push(number_character);
                        characters.next();
                    } else if number_character == '.' {
                        if decimal_point_found {
                            return Err("Cannot have two decimal points in number")
                        } else {
                            number.push(number_character);
                            characters.next();
                            decimal_point_found = true;
                        }
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Number(number));
                last_token_was_value = true;
                if close_bracket {
                    tokens.push(Token::CloseBracket);
                    close_bracket = false;
                }
            },
            _ => return Err("Unexpected character in conditions")
        }
    }
    return Ok(tokens)
}

fn test_execute_component<'a>(instructions_iter: &mut impl Iterator<Item = &'a u64>) -> Result<Vec<u64>, &'static str> {
    let component_code = instructions_iter.next().ok_or("expected component")?;
    let number_of_component_parameters = Spell::get_number_of_component_parameters(component_code);
    let mut parameters: Vec<u64> = vec![];
    for _ in 0..number_of_component_parameters {
        let parameter = *instructions_iter.next().ok_or("expected parameter")?;

        match parameter {
            TRUE..=FALSE => parameters.push(parameter),
            NUMBER_LITERAL => {
                parameters.push(parameter);
                parameters.push(*instructions_iter.next().ok_or("Expected number after number literal opcode")?);
            },
            COMPONENT => parameters.extend(test_execute_component(instructions_iter)?),
            _ => return Err("Invalid parameter")
        }
    }

    return match COMPONENT_TO_FUNCTION_MAP.get(component_code) {
        Some((_, _, return_type)) => {
            match *return_type {
                ReturnType::Float => Ok(vec![NUMBER_LITERAL, 0]),
                ReturnType::Boolean => Ok(vec![TRUE]),
                ReturnType::None => return Err("Expected return from component")
            }
        },
        None => return Err("Component does not exist")
    };
}

/// Does a mock execution of the if statement logic where components are all evaulated to default return values and aren't actually run
fn test_logic(logic: &Vec<u64>) -> Result<(), &'static str> {
    let mut instructions_iter = logic.iter();
    let mut rpn_stack: Vec<u64> = Vec::new();
    while let Some(&if_bits) = instructions_iter.next() {
        match if_bits {
            END_OF_SCOPE => break,
            TRUE..=FALSE => rpn_stack.push(if_bits), // true and false
            NUMBER_LITERAL => rpn_stack.extend(vec![NUMBER_LITERAL, *instructions_iter.next().ok_or("Expected following value")?]), // if 102, next bits are a number literal
            COMPONENT => rpn_stack.extend(test_execute_component(&mut instructions_iter)?), // Component
            AND => rpn_operations::binary_operation(&mut rpn_stack, boolean_logic::and)?, // And statement
            OR => rpn_operations::binary_operation(&mut rpn_stack, boolean_logic::or)?, // Or statement
            NOT => { // Not statement
                let bool_one = rpn_stack.pop().ok_or("Expected value to compare")?;
                rpn_stack.push(boolean_logic::not(bool_one)?);
            },
            XOR => rpn_operations::binary_operation(&mut rpn_stack, boolean_logic::xor)?, // Xor statement
            EQUALS => { // Equals statement
                let argument_two = rpn_stack.pop().ok_or("Expected value to compare")?;
                let opcode_or_bool = rpn_stack.pop().ok_or("Expected value to compare")?;
                if opcode_or_bool == NUMBER_LITERAL {
                    let argument_one = f64::from_bits(rpn_stack.pop().ok_or("Expected value to compare")?);
                    let _ = rpn_stack.pop().ok_or("Expected number literal opcode")?;
                    if argument_one == f64::from_bits(argument_two) {
                        rpn_stack.push(TRUE);
                    } else {
                        rpn_stack.push(FALSE);
                    }
                } else {
                    if opcode_or_bool == argument_two {
                        rpn_stack.push(TRUE);
                    } else {
                        rpn_stack.push(FALSE);
                    }
                }
            },
            GREATER_THAN => rpn_operations::compare_operation(&mut rpn_stack, |a, b| a > b)?, // Greater than
            LESSER_THAN => rpn_operations::compare_operation(&mut rpn_stack, |a, b| a < b)?, // Lesser than
            MULTIPLY => rpn_operations::maths_operation(&mut rpn_stack, |a, b| a * b)?, // Multiply
            DIVIDE => rpn_operations::maths_operation(&mut rpn_stack, |a, b| a / b)?, // Divide
            ADD => rpn_operations::maths_operation(&mut rpn_stack, |a, b| a + b)?, // Add
            SUBTRACT => rpn_operations::maths_operation(&mut rpn_stack, |a, b| a - b)?, // Subtract
            POWER => rpn_operations::maths_operation(&mut rpn_stack, |a, b| a.powf(b))?, // Power
            _ => return Err("Opcode doesn't exist")
        }
    }
    Ok(())
}

fn parse_logic(conditions: &str) -> Result<Vec<u64>, &'static str> {
    // Uses the Shunting Yard Algorithm to turn player written infix code into executeable postfix (RPN) code
    let mut holding_stack: Vec<String> = vec![];
    let mut output: Vec<String> = vec![];

    for token in tokenise(conditions)? {
        match token {
            Token::Opcode(opcode) => {
                while let Some(operator) = holding_stack.last() {
                    if get_precedence(operator) > get_precedence(&opcode) ||
                        (get_precedence(operator) == get_precedence(&opcode) && get_associative_direction(&opcode) == Direction::Left) {
                            output.push(holding_stack.pop().unwrap());
                        } else {
                            break;
                        }
                }
                holding_stack.push(opcode);
            },
            Token::OpenBracket => {
                holding_stack.push("(".to_string())
            },
            Token::CloseBracket => {
                let mut operator = holding_stack.pop().ok_or("Expected opening bracket")?;
                while operator != "(" {
                    output.push(operator);
                    operator = holding_stack.pop().ok_or("Expected opening bracket")?;
                }
            },
            Token::Boolean(boolean) => {
                output.push(boolean);
            },
            Token::Number(num) => {
                if let Ok(_) = num.parse::<f64>() {
                    output.push(num);
                } else {
                    return Err("Invalid condition")
                }
            }
            Token::Component(component) => {
                output.push(component)
            }
        }
    }
    // Pop remaining operators off holding stack and push to output
    for _ in 0..holding_stack.len() {
        output.push(holding_stack.pop().ok_or_else(|| "Expected to work: Program logic fault")?);
    }
    let mut bit_conditions: Vec<u64> = vec![];
    for condition in output {
        match condition.as_str() {
            "and" => bit_conditions.push(AND),
            "or" => bit_conditions.push(OR),
            "not" => bit_conditions.push(NOT),
            "xor" => bit_conditions.push(XOR),
            "==" | "=" => bit_conditions.push(EQUALS),
            ">" => bit_conditions.push(GREATER_THAN),
            "<" => bit_conditions.push(LESSER_THAN),
            "*" => bit_conditions.push(MULTIPLY),
            "/" => bit_conditions.push(DIVIDE),
            "+" => bit_conditions.push(ADD),
            "-" => bit_conditions.push(SUBTRACT),
            "^" => bit_conditions.push(POWER),
            "true" => bit_conditions.push(TRUE),
            "false" => bit_conditions.push(FALSE),
            number if number.parse::<f64>().is_ok() => {
                bit_conditions.push(NUMBER_LITERAL); // Indicates number literal
                bit_conditions.push(number.parse::<f64>().unwrap().to_bits());
            }
            possible_component => {
                // Attempt to see if is a component
                if possible_component.ends_with(')') && !possible_component.starts_with('(') && possible_component.contains('(') {
                    bit_conditions.extend(parse_component(possible_component)?);
                } else {
                    return Err("Invalid condition")
                }
            }
        }
    }
    match test_logic(&bit_conditions) {
        Ok(_) => Ok(bit_conditions),
        Err(error) => Err(error)
    }
}

fn parse_component(component_call: &str) -> Result<Vec<u64>, &'static str> {
    let mut component_vec: Vec<u64> = vec![103];
    let (component_name, parameters) = parse_component_string(component_call)?;
    let component_num = match get_component_num(&component_name) {
        Some(num) => num,
        None => return Err("Invalid component: mapping doesn't exist")
    };
    component_vec.push(component_num);
    for parameter in parameters {
        component_vec.extend(parameter.to_bits()?)
    }
    return Ok(component_vec)
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
            found_opening_bracket = true;
            break;
            // Checking if character is alphabetic if not an open bracket.
        } else if !character.is_alphabetic() && character != '_' {
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
    Float(f64),
    Boolean(bool),
    Component(String)
}

impl Parameter {
    fn to_bits(&self) -> Result<Vec<u64>, &'static str> {
        match self {
            Parameter::Float(float) => Ok(vec![NUMBER_LITERAL, float.to_bits()]),
            Parameter::Boolean(boolean) => match boolean {
                true => Ok(vec![TRUE]),
                false => Ok(vec![FALSE])
            },
            Parameter::Component(component) => parse_component(&component)
        }
    }
}

fn collect_parameters(parameters_string: &str, component_name: &str) -> Result<Vec<Parameter>, &'static str> {
    let mut parameter = String::new();
    let mut parameters: Vec<Parameter> = vec![];

    let mut index = 0;

    if let Some((_, encoded_types, _)) = COMPONENT_TO_FUNCTION_MAP.get(&get_component_num(component_name).ok_or("Component doesn't exist")?) {
        let encoded_types: &[u64] = encoded_types;
        for character in parameters_string.chars() {
            if character != ',' {
                parameter.push(character);
                continue
            }

            if parameter.is_empty() {
                return Err("Invalid parameters: Must have value before bracket")
            }

            if index >= encoded_types.len() {
                return Err("Invalid parameters: More parameters than expected types");
            }

            // Adding parameter to parameters vector
            parameters.push(parse_parameter(&parameter, encoded_types[index])?);
            index += 1;

            // Clear parameter string so next one can be recorded
            parameter.clear();

        }

        // Adding last parameter
        if !parameter.is_empty() {
            if index >= encoded_types.len() {
                return Err("Invalid parameters: More parameters than expected");
            }
            parameters.push(parse_parameter(&parameter, encoded_types[index])?);
        }

        if parameters.len() < encoded_types.len() {
            return Err("Invalid parameters: Missing parameters")
        } else if parameters.len() > encoded_types.len() {
            return Err("Invalid parameters: More parameters than expected")
        }

    } else {
        panic!("Expected component mapping")
    }

    return Ok(parameters)
}

fn parse_parameter(parameter_string: &str, parameter_type: u64) -> Result<Parameter, &'static str> {
    let trimmed_parameter_string = parameter_string.trim();

    // Check if component
    if trimmed_parameter_string.ends_with(")") {
        return Ok(Parameter::Component(trimmed_parameter_string.to_string()))
    }

    match parameter_type {
        1 => Ok(Parameter::Float(trimmed_parameter_string.parse::<f64>().map_err(|_| "Couldn't parse parameter: should be float")?)),
        2 => Ok(Parameter::Boolean(trimmed_parameter_string.parse::<bool>().map_err(|_| "Couldn't parse parameter: should be boolean")?)),
        _ => Err("Invalid parameters: parameter doesn't match expected type")
    }
}

fn parse_about_line(equation: &str) -> Result<Vec<u64>, &'static str>{
    let (name, value) = equation.split_once('=').ok_or_else(|| "There must be an equals sign in an about line")?;
    
    match (name.trim(), value.trim()) {
        ("colour", values) | ("color", values) => {
            let mut opcodes = vec![0];
            opcodes.extend(match match values.strip_prefix('[')
            .and_then(|x| x.strip_suffix(']'))
            .ok_or_else(|| "Invalid parameters: should be a list and have \"[\" \"]\"")?
            .split(',')
            .map(str::trim)
            .map(str::parse::<f32>)
            .collect::<Result<Vec<f32>, _>>()
            .map_err(|_| "Invalid parameters: should be floating point numbers (with decimal point)")?[..] {
                [a, b, c] => [a, b, c],
                _ => {
                    return Err("Invalid number of arguments: color attribute only has 3 values")
                }
            }.into_iter().filter(|x| (0.0..=1.0).contains(x)).collect::<Vec<f32>>()[..]{
                [a, b, c] => [a, b, c],
                _ => {
                    return Err("Invalid values: arguments should be between 0 and 1")
                }
            }.into_iter()
            .map(|x| f64::to_bits(x as f64))
            .collect::<Vec<u64>>());
            Ok(opcodes)
        },
        _ => Err("Unkown attribute: undefined attribute")
    }
}

// Tests to check that the library is working properly
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_emtpy_spell() {
        assert_eq!(parse_spell(""), Ok(vec![]));
    }

    #[test]
    fn parse_basic_booleans() {
        assert_eq!(parse_logic("true and false or true"), Ok(vec![TRUE, FALSE, AND, TRUE, OR]));
    }

    #[test]
    fn parse_basic_spell() {
        assert_eq!(parse_spell("when_created:\ngive_velocity(1, 1, 1)"), Ok(vec![READY_SECTION, COMPONENT, GIVE_VELOCITY, NUMBER_LITERAL, f64::to_bits(1.0), NUMBER_LITERAL, f64::to_bits(1.0), NUMBER_LITERAL, f64::to_bits(1.0)]))
    }

    #[test]
    fn parse_basic_repeat() {
        assert_eq!(parse_spell("repeat:\ngive_velocity(1,1,1)"), Ok(vec![PROCESS_SECTION, NUMBER_LITERAL, f64::to_bits(1.0), COMPONENT, GIVE_VELOCITY, NUMBER_LITERAL, f64::to_bits(1.0), NUMBER_LITERAL, f64::to_bits(1.0), NUMBER_LITERAL, f64::to_bits(1.0)]))
    }

    #[test]
    fn parse_advanced_repeat() {
        assert_eq!(parse_spell("repeat every 2:\ngive_velocity(0,0,0)"), Ok(vec![PROCESS_SECTION, NUMBER_LITERAL, f64::to_bits(2.0), COMPONENT, GIVE_VELOCITY, NUMBER_LITERAL, 0, NUMBER_LITERAL, 0, NUMBER_LITERAL, 0]))
    }

    #[test]
    fn parse_advanced_repeat_with_irregular_spacing() {
        assert_eq!(parse_spell("repeat  every      3:\ngive_velocity(0,0,0)"), Ok(vec![PROCESS_SECTION, NUMBER_LITERAL, f64::to_bits(3.0), COMPONENT, GIVE_VELOCITY, NUMBER_LITERAL, 0, NUMBER_LITERAL, 0, NUMBER_LITERAL, 0]))
    }


    #[test]
    fn parse_spell_color() {
        assert_eq!(parse_about_line("color = [0.4, 0, 0.8]"), Ok(vec![COLOR, f64::to_bits((0.4 as f32) as f64), 0, f64::to_bits((0.8 as f32) as f64)]));
        assert_eq!(parse_about_line("colour = [0.4, 0, 0.8]"), Ok(vec![COLOR, f64::to_bits((0.4 as f32) as f64), 0, f64::to_bits((0.8 as f32) as f64)]));
    }
    
    #[test]
    fn parse_invalid_spell_color() {
        assert_eq!(parse_about_line("color = [0.212, 1, 2.3]"), Err("Invalid values: arguments should be between 0 and 1"));
        assert_eq!(parse_about_line("color = 0.4, 0,284]"), Err("Invalid parameters: should be a list and have \"[\" \"]\""));
        assert_eq!(parse_about_line("color = [0.4, 0,284"), Err("Invalid parameters: should be a list and have \"[\" \"]\""));
        assert_eq!(parse_about_line("color = [a, 0,284]"), Err("Invalid parameters: should be floating point numbers (with decimal point)"));
    }

    #[test]
    fn parse_spell_color_with_irregular_spacing() {
        assert_eq!(parse_about_line("     color      =        [   0.212,    1,0.3]"), Ok(vec![COLOR, f64::to_bits((0.212 as f32) as f64), f64::to_bits((1 as f32) as f64), f64::to_bits((0.3 as f32) as f64)]));
    }

    #[test]
    fn parse_about_section(){
        assert_eq!(parse_spell("about:\ncolour = [0.4, 0, 0.8]"), Ok(vec![METADATA_SECTION, COLOR, f64::to_bits((0.4 as f32) as f64), 0, f64::to_bits((0.8 as f32) as f64)]))
    }

    #[test]
    fn parse_if_statement_spell() {
        assert_eq!(parse_spell("when_created:\nif false {\ngive_velocity(1, 0, 0)\n}"), Ok(vec![READY_SECTION, IF, FALSE, 0, COMPONENT, GIVE_VELOCITY, NUMBER_LITERAL, f64::to_bits(1.0), NUMBER_LITERAL, 0, NUMBER_LITERAL, 0, 0]))
    }

    #[test]
    fn parse_advanced_if_statement_spell() {
        assert_eq!(parse_spell("when_created:\nif false or get_time() > 5 {\ngive_velocity(1, 0, 0)\n}"), Ok(vec![READY_SECTION, IF, FALSE, COMPONENT, GET_TIME, NUMBER_LITERAL, f64::to_bits(5.0), GREATER_THAN, OR, 0, COMPONENT, GIVE_VELOCITY, NUMBER_LITERAL, f64::to_bits(1.0), NUMBER_LITERAL, 0, NUMBER_LITERAL, 0, 0]))
    }

    #[test]
    fn parse_component_as_parameter() {
        assert_eq!(parse_spell("when_created:\ngive_velocity(get_time(), 0, 0)"), Ok(vec![READY_SECTION, COMPONENT, GIVE_VELOCITY, COMPONENT, GET_TIME, NUMBER_LITERAL, 0, NUMBER_LITERAL, 0]))
    }

    #[test]
    fn parse_complex_spell() {
        assert_eq!(parse_spell("about:\ncolor = [1, 0, 1]\n\nwhen_created:\ngive_velocity(1, 0, 0)\n\nrepeat every 5:\ngive_velocity(0.1, 0, 0)"), Ok(vec![METADATA_SECTION, COLOR,f64::to_bits(1.0),0,f64::to_bits(1.0),READY_SECTION,COMPONENT,GIVE_VELOCITY,NUMBER_LITERAL,f64::to_bits(1.0),NUMBER_LITERAL,0,NUMBER_LITERAL,0,PROCESS_SECTION,NUMBER_LITERAL,f64::to_bits(5.0),COMPONENT,GIVE_VELOCITY,NUMBER_LITERAL,f64::to_bits(0.1),NUMBER_LITERAL,0,NUMBER_LITERAL,0]))
    }

    /// Ensures all components in the COMPONENT_TO_NUM_MAP are in the COMPONENT_TO_FUNCTION_MAP
    #[test]
    fn compare_component_maps() {
        for value in COMPONENT_TO_NUM_MAP.values() {
            match COMPONENT_TO_FUNCTION_MAP.get(value) {
                Some(_) => {},
                None => panic!("Component code {} not in COMPONENT_TO_FUNCTION_MAP", value)
            }
        }
    }
}
