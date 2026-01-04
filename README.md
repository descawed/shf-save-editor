# shf-save-editor

A save editor for Silent Hill f.

## Usage

Download the latest release from the [releases tab](https://github.com/descawed/shf-save-editor/releases) and run `shf-save-editor.exe`. Use File > Open... or click the
"Open .sav..." button to open a save file. Saves are located in `%LocalAppData%\SHf\Saved\SaveGames` in the folder named
with a bunch of numbers (not sure if this is always the same or different for each person). Once you're satisfied with
your changes, use File > Save to save them. I recommend making a backup before replacing a save file as the editor is
still experimental.

## Editing

The editor has two views: Simple and Advanced.

### Simple

The simple view allows convenient editing of the most common information. You can edit the following in the simple view:

- Difficulty
- Health
- Stamina
- Sanity
- Faith
- Omamori
- Consumables
- Key Items
- Letters

**IMPORTANT NOTE**: Be careful giving yourself key items and letters that you shouldn't have yet. This can cause them
not to spawn in their proper locations and softlock you. For example, at the school, if you already have the Unopened
Envelope in your inventory when you enter the room with the puzzle box, it won't spawn, and you'll be unable to interact
with the puzzle and escape the room.

Also note that the save only stores the ratio of your current stamina and sanity to the maximum values, not your actual
current amount of stamina and sanity, so you can only edit those values via the ratio sliders. For health, the save
contains both the number and the ratio, so you can edit using either one.

### Advanced

The advanced view displays a tree view of the UE5 objects that make up the save file. Functionality is fairly
basic – you can edit the names and values of most fields and delete struct properties and array elements. Inserting new
properties/elements is currently limited to scalar types. The editor will also allow you to edit the types of objects,
but I don't recommend it; it doesn't properly update things behind the scenes. Some types are not properly decoded yet
and can't be viewed or edited.

As far as finding something useful to edit, most player-related information is in `PlayerStateRecord` and
`HinakoRecord`. Beyond that, you're pretty much on your own; I honestly don't know what most of the rest of this data
controls in-game. Also note that there is no undo. If you delete something by accident, you'll just have to close the
editor and start over.

## Credits

This tool was made by [descawed](https://github.com/descawed). Shout out to the following tools/libraries I used which
helped me understand the save format:
- [uesaveeditor.cc](https://uesaveeditor.cc/)
- [UeSaveGame](https://github.com/CrystalFerrai/UeSaveGame)