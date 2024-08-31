[comment]: <> (If you can see this text, you're not using a text editor that can interpret markdown (this type of document))

Spell code is split into three sections: `when_created`, `repeat` (which are the equivalent of `_ready` and `_physics_process` in Godot), and `about` (which sets the attributes of a spell). In the first two sections you can write components and if statements. An example of spell code would be:

```
repeat:
if true {
give_velocity(1, 0, 0)
}
```

If statements require curly brackets to indicate where they start and stop. The opening curly bracket must be the last character of the if statement and the closing bracket must be on a line by itself.

Note that new lines are needed for the interpretation of spell code, so if you try and type `repeat: give_velocity(1, 0, 0)` all on one line, it won't work. You can get around this using the new line character `\n`. So instead you would write `repeat:\n give_velocity(1, 0, 0)` if you want to write your spell code all on one line.
