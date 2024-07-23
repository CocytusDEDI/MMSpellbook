var number_of_spells_to_make = 1
var number_of_spells = 0

var efficiencies_bytecode = {}

var example_instructions = "
when_created:
if 5 > 3 and true {
give_velocity(1, 0, 0)
}
"
var example_spell_energy = 10.0

func _process(delta):
    # Just to make sure infinite spells are made. Replace with your own code that chooses when to cast a spell.
    if number_of_spells < number_of_spells_to_make:
        number_of_spells += 1

        # Spell code:
        # Creates the spell
        var spell = Spell.new()
        # Allows for efficiencies to be updated
        spell.connect_player(self)
        # Sets the spells inital energy
        spell.set_energy(example_spell_energy)
        # Gives the spell instructions. Spell instructions need to be in bytecode format so they are converted first
        var instructions = Spell.get_bytecode_instructions(example_instructions)
        # Sets the spells position to be the same as the players
        spell.set_position(self.global_position)
        # Put the spell into the game
        get_tree().root.add_child(spell)

# Called by the spell to update the component's efficiency
func update_component_efficiency(component, efficiency_increase):
    if str(component) in efficiencies_bytecode:
        efficiencies_bytecode[str(component)] += efficiency_increase
    else:
        efficiencies_bytecode[str(component)] = 1 + efficiency_increase
