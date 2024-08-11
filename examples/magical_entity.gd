extends MagicalEntity


func _ready():
    # Adds component to the components the player is allowed to cast. This code should be moved from the ready statement and to wherever you give the player components.
    Spell.add_component("give_velocity")

func _process(delta):
    self.handle_player_spell_casting(delta)

func set_spell(text):
    # Attempts to translate the instructions into executable format
    var instructions_result = Spell.get_bytecode_instructions(text)

    # Splits instructions_result into a instructions variable and successful variable
    var instructions = instructions_result.get("instructions") # json list
    var successful = instructions_result.get("successful") # boolean
    var error_message = instructions_result.get("error_message") # string

    # Test to see if the player as access to the components they're trying to cast
    var allowed_to_cast_result = Spell.check_allowed_to_cast(instructions)

    # Splits the allowed_to_cast_result into an allowed_to_cast variable and denial_reason variable
    var allowed_to_cast = allowed_to_cast_result.get("allowed_to_cast") # boolean
    var denial_reason = allowed_to_cast_result.get("denial_reason") # denial reason

    # If spell was successfully translated and they're allowed to cast it
    if successful and allowed_to_cast:
        # Gives the spell the users instructions
        self.set_instructions(instructions)

    # If spell translation was unsuccesful, do something
    else:
        if !successful:
            print(error_message)
        else:
            print(denial_reason)
