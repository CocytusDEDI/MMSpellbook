[comment]: <> (If you can see this text, you're not using a text editor that can interpret markdown. If you wish to keep reading regardless, ignore any ` characters you see in this document.)

Spell code is split into two sections: `on_creation` and `repeat` (which are the equivilent of `_ready` and `_physics_process` in Godot). In the sections you can write components and if statements (though if statements aren't fully supported yet). An example of spell code would be:

```
repeat:
give_velocity(1, 0, 0)
```

Note that new lines are needed for the interpretation of spellcode, so if you try and type `repeat: give_velocity(1, 0, 0)` all on one line, it won't work, you can get around this using the new line character `\n`. So instead you would write `repeat:\n give_velocity(1, 0, 0)` if you want to write your spell code all on one line.