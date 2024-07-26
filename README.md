# MMSpellbook
MMSpellbook (Magic Modelling Spellbook) is a magic system for Godot that allows for the creation of custom spells. Once complete, it should provide a way for players to write their own custom spells using a simple language in game and then be executed. The spell has an amount of energy and that energy is used as the spell executes. Different instructions in the spell have different efficiencies which is a factor in how much energy that instruction uses. The spell executes until it runs out of energy.

## How to use
- Download the repository. You can the command `git clone https://github.com/CocytusDEDI/MMSpellbook.git` in the terminal if you have git installed
- Put `MMSpellbook.gdextension` into your godot project and change the paths to where the compiled library would be
- Compile the rust code using `cargo build` while in the repository folder (if you don't have rust installed, install it from the rust website)
- Use the code in `player.gd` example in the examples folder. This code should be used alongside your already existing player code

## Missing features / Issues / Planning to change
- Only one component exists
- Custom materials and meshes can't be loaded for spells

## Debugging
Start Godot via the terminal so you can see detailed error messages.
