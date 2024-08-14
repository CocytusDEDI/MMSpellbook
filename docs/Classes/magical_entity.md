# Magical Entity
Inherits CharacterBody3D

## Properties (Incomplete list)
- health: float
- shield: float

## Methods (Incomplete list)
- add_restricted_component(component: String, parameter_restrictions: String)
    - Adds a component to the component catalogue with restrictions on what parameters the player enter. The `parameter_restrictions` are a json list. All values must be strings. A parameter restriction can be a number (e.g. `"5"`), a range of numbers (e.g. `"0-3"`), `"true"` or `"false"` or the `"ANY"` keyword. An example for the format would be `"[[\"4\", \"6-7\"], [\"ANY\"]]"`. Note that you have to use the delimiter character `\` if you are typing it directly into the editor to prevent it from thinking you're ending the string.