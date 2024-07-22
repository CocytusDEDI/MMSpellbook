# MMSpellbook
MMSpellbook (Magic Modelling Spellbook) is a magic system for Godot that allows for the creation of custom spells. Once complete, it should provide a way for players to write their own custom spells using a simple language in game and then be executed. The spell has an amount of energy and that energy is used as the spell executes. Different instructions in the spell have different efficencies which changes how much energy is used per instruction in the spell. The spell executes until it runs out of energy.

## How to use
- Git clone the repository using `git clone https://github.com/CocytusDEDI/MMSpellbook.git` in the terminal
- Put `MMSpellbook.gdextension` into your godot project and change the paths to where the compiled library would be
- Compile the rust code using `cargo build` while in the repository folder (if you don't have rust installed, install it from the rust website)
- Write GDScript code to interact with spell code
    - Create a new spell using `Spell.new()`
    - Create a cast count dictionary for each component with the component name as the key and the value being the number of casts. This effects the efficiency of the components being cast. The number of casts should be greater than zero, even if it has never been cast before. You should pass the dictionary to the `.set_cast_counts()` method of your spell
    - Give the spell energy with `.set_energy()`
    - Create instructions for the spell (you can follow the documentation for the formatting) and then give those instructions to the method `.get_bytecode_instructions()` which will return the executable spell code in byte code format. This byte code can then be given to the method `.set_instructions()` to give the spell the instructions
    - Make the spell a child of a none moving object (not the player) and give the spell the initial position of the player. You can find the initial position of the player with the `.global_position()` method (has to be used on the player not the spell) and then give that position to the spell use the method `.set_position()` on your spell
- Run the game

## Missing features / Issues
- Efficiencies aren't updated after casting
- No error handling between GDScript and MMSpellbook

## Debugging
Start Godot via the terminal so you can see detailed error messages.
