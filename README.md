# MMSpellbook
MMSpellbook (Magic Modelling Spellbook) is a magic system for Godot that allows for the creation of custom spells. The player can write their own spells that can be compiled and executed. When executed, the spell is given energy which is used as the spell executes. Different instructions in the spell have different efficiencies which is a factor in how much energy that instruction uses. The spell executes until it runs out of energy.

## How to use
- Download the repository. You can the command `git clone https://github.com/CocytusDEDI/MMSpellbook.git` in the terminal if you have git installed or you can download the latest release on the releases page
- Put `MMSpellbook.gdextension` into your Godot project and change the paths to where the compiled library would be
- Compile the rust code using `cargo build` while in the repository folder (if you don't have rust installed, install it from the rust website)
- Choose how you want to use MMSpellbook
	- MagicalEntity (**Recommend**): This is for if you want to use MMSpellbook as your core magic system, and want to deal damage with it. 
	- Spell: You can use `Spell` without `MagicalEntity` if you don't want to deal damage with it and don't want to use MMSpellbook as your core magic system
- Both `Spell` and `MagicalEntity` are classes that you will be able to use in the editor and call their methods to interact with them. You can check out [Commune](https://github.com/CocytusDEDI/Commune) as an example implementation.

## Optional setup (but recommend)
- Forms: This feature is designed to give the game developer more freedom in spell variety and give the spell custom visuals. Forms give the player the ability to call the components `take_form` and `undo_form` which allow a single scene to be added as a child of the spell. These scenes are specified by the game developer in the `config.toml` file under the `[forms]` section. You can checkout `examples/config.toml` for an example. Keep in mind that the `config.toml` file should be placed in a folder called `Spell` in the `res://` directory.

## Debugging
Start Godot via the terminal so you can see detailed error messages.
