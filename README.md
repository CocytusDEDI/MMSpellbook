# MMSpellbook
MMSpellbook (Magic Modelling Spellbook) is a magic system for Godot that allows for the creation of custom spells. Once complete, it should provide a way for players to write their own custom spells using a simple language in game and then be executed. The spell has an amount of energy and that energy is used as the spell executes. Different instructions in the spell have different efficencies which changes how much energy is used per instruction in the spell. The spell executes until it runs out of energy.

## How to run code:
- Git clone the repository using `git clone https://github.com/CocytusDEDI/MMSpellbook.git` in the terminal
- Put `MMSpellbook.gdextension` into your godot project and change the paths to where the compiled library would be.
- Must manually enter instructions into the rust code. This can be done in the `init` function. Either change the ready instructions or process instructions depending on if you want the code to be ran when the spell is created or on repeat. An example would be `crate::spelltranslator::parse_spell("give_velocity(1, 1, 1)")`.
- Efficency HashMap must also be manually entered into rust code (Also in the init function)
- Compile the rust code using `cargo build` while in the repository folder (if you don't have rust installed, install it from the rust website)
- Put a spell node into the scene tree
- Run the game

## Missing features
- GDScript for making spells
- updating efficencies after casting
- if statement support
- use of numbers in if statements (currently just bools)
