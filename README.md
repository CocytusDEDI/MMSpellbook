# MMSpellbook
MMSpellbook (Magic Modelling Spellbook) is a magic system for Godot that allows for the creation of custom spells. Once complete, it should provide a way for players to write their own custom spells using a simple language in game and then be executed. The spell has an amount of energy and that energy is used as the spell executes. Different instructions in the spell have different efficencies which changes how much energy is used per instruction in the spell. The spell executes until it runs out of energy.

## How to run code:
- Git clone the repository using `git clone https://github.com/CocytusDEDI/MMSpellbook.git` in the terminal
- Put `MMSpellbook.gdextension` into your godot project and change the paths to where the compiled library would be.
- Compile the rust code using `cargo build` while in the repository folder (if you don't have rust installed, install it from the rust website)
- Write GDScript code to interact with spell code:
    - Create a new spell using `Spell.new()`
    - Create an efficiency dictionary of component names to efficiency values (real numbers) and give it to the spell using the method `.give_efficiencies()` on your spell 
    - Create instructions for the spell (you can follow the documentation for the formatting) and then give those instructions to the method `.get_instructions()` which will return a json list containing the spell code. This spell code can then be given to the method `.give_instructions()` to give the spell the instructions.
    - Make the spell a child of a none moving object (not the player) and give the spell the initial position of the player. You can find the initial position of the player with the `.global_position()` method (has to be used on the player not the spell) and then give that position to the spell use the method `.set_position()` on your spell.
- Run the game

## Missing features
- updating efficiencies after casting
- if statement support
- use of numbers in if statements (currently just bools)
