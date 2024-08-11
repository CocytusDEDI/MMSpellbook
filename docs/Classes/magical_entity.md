# Magical Entity
Magical Entity handles the casting of spells and can take damage from spells.

## How to use
### For a Player
First make a new `MagicalEntity` node. The `MagicalEntity` node replaces the `CharacterBody3D` node that you would usually use while making a player. Attach a script to this node. You can use the CharacterBody3D template for this. At the top of the script should be the line `extends MagicalEntity`. In your `_process` function you can use the line `self.handle_player_spell_casting(delta)`. To make this line work through you will need to make a new action in the input map in your project settings. This action should be called `cast`. The player holds cast down to charge spells and releases the fire them. To fire a spell, the player must first load a spell. To do so we must first translate the spell from player written code to executeable code:
```
# Attempts to translate the instructions into executable format
var instructions_result = Spell.get_bytecode_instructions(text)

# Splits instructions_result into a instructions variable and successful variable
var instructions = instructions_result.get("instructions") # json list
var successful = instructions_result.get("successful") # boolean
var error_message = instructions_result.get("error_message") # string
```

The `text` variable that is given to `get_bytecode_instructions` is the player written code and the `instructions` variable is that code compiled into executable spell code. This code can then be loaded as the current spell with the line `self.set_instructions(instructions)` where the instructions are the executable spell code. Once the instructions are loaded they will be cast whenever the `cast` action is released.

To allow for progression, you can use the `check_allowed_to_cast` method. This checks that all of the components the player has written they are allowed to use. The list of components a player is allowed to use is stored by the `component_catalogue`. To add to the player catalogue you can call the method `add_component` and pass in the components name to give the player access to that component. The following code can be used to check that the player is allowed to cast the spell:
```
# Test to see if the player as access to the components they're trying to cast
var allowed_to_cast_result = self.check_allowed_to_cast(instructions)

# Splits the allowed_to_cast_result into an allowed_to_cast variable and denial_reason variable
var allowed_to_cast = allowed_to_cast_result.get("allowed_to_cast") # boolean
var denial_reason = allowed_to_cast_result.get("denial_reason") # denial reason
```
If `allowed_to_cast` you should set the instructions, if not, you shouldn't set the instructions. If you want to store the `component_catalogue` you can use the method `save_component_catalogue` and you can use the method `load_data` to load the component catalogue. You should make sure to save the component catalogue whenever the player is given a new component if you want them to keep it.