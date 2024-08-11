# Magical Entity
Magical Entity handles the casting of spells and can take damage from spells.

## How to use
### For a Player
To start, make a new `MagicalEntity` node. The `MagicalEntity` node replaces the `CharacterBody3D` node that you would usually use while making a player. Now attach a script to this node, you can use the CharacterBody3D template for this, just make sure that at the top of the script there is the line `extends MagicalEntity`. 

To handle the charging and creation of spells for you, you can use the line `self.handle_player_spell_casting(delta)` in your `_process` function. To make this line work through you will need to make a new action in the input map in your project settings called `cast`. You can map this to whatever button you want. The player holds the `cast` action/button down to charge spells and releases it to cast them. 

If you tried this out, you might notice the spell isn't doing anything... that's because you need to give it instructions! Instructions are written by the player and then translated into executable spell code. The following code can be used to translate spells:

```
# Attempts to translate the instructions into executable format
var instructions_result = Spell.get_bytecode_instructions(player_written_spell)

# Splits instructions_result into a instructions variable and successful variable
var instructions = instructions_result.get("instructions") # json list
var successful = instructions_result.get("successful") # boolean
var error_message = instructions_result.get("error_message") # string
```

The translated code can then be set with the line `self.set_loaded_spell(instructions)` where the instructions are the executable spell code. Once the instructions are loaded they will be cast whenever the `cast` action is released.

You may see the problem that the player can cast any component they want to right now. To prevent this, you can use the `check_allowed_to_cast` method. This checks that the player is allowed to use all the components they wrote in the spell. The list of components a player is allowed to use is stored by the `component_catalogue`. To add to the component catalogue you can call the method `add_component` and pass in the components name to give the player access to that component. This will allow the player to cast the component with any parameters they want. If you'd like to restrict the parameters they can use (for example, for the `take_form` method), you can use the `add_restricted_component` method instead. A guide on the format for the parameter restrictions can be found in `magical_entity.md`. The following code can be used to check that the player is allowed to cast the spell:

```
# Test to see if the player as access to the components they're trying to cast
var allowed_to_cast_result = self.check_allowed_to_cast(instructions)

# Splits the allowed_to_cast_result into an allowed_to_cast variable and denial_reason variable
var allowed_to_cast = allowed_to_cast_result.get("allowed_to_cast") # boolean
var denial_reason = allowed_to_cast_result.get("denial_reason") # denial reason
```

If `allowed_to_cast` is true you should set the instructions, if not, you shouldn't.

The component catalogue is stored in memory, and so will be deleted if not saved. Before you can save it you must set the save path for the MagicalEntity using the `set_save_path` method. The path is local, so don't use absolute paths or Godot paths like 'user://'. Once the save path is set, you can now save and load data, and in this case we want to save the component catalogue with the method `save_component_catalogue`. You can load up the MagicalEntity's save data (which includes the component catalogue) with the method `load_saved_data`.