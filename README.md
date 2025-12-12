# shf-save-editor

A bare-bones, low-level save editor for Silent Hill f.

## Usage

Download the latest release from the [releases tab](https://github.com/descawed/shf-save-editor/releases) and run
`shf-save-editor.exe`. Use File > Open .sav... or click the "Open .sav..." button to open a save file. Saves are located
in `%localappdata%\SHf\Saved\SaveGames` in the folder named with a bunch of numbers (not sure if this is always the same
or different for each person).

The editor displays a tree view of the UE5 objects that make up the save file. Functionality is fairly basic – you can
edit the names and values of most fields and delete struct properties and array elements. Inserting new
properties/elements is not implemented. The editor will also allow you to edit the types of objects, but I don't
recommend it; it doesn't properly update things behind the scenes. Many types are not properly decoded yet and can't be
viewed or edited. As far as finding something useful to edit, you're pretty much on your own; I honestly don't know what
most of this data controls in-game. Some things are fairly obvious from the names (for example, `HinakoRecord` >
`Health` is Hinako's health), so it's mostly a matter of poking around the data until you find something interesting.
Also note that there is no undo. If you delete something by accident, you'll just have to close the editor and start
over.

Once you're satisfied with your changes, use File > Save as... to save them. This will prompt you for the location to
save the edited save file to. I recommend making a backup before replacing a save file as the editor is still experimental
and may corrupt your save file.