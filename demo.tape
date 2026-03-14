Output assets/demo.gif

Set Shell "fish"

Set Theme "nord"

Set FontSize 16
Set FontFamily "FiraCode Nerd Font"

Set Width 1200
Set Height 700

Set Padding 40
Set Margin 40
Set MarginFill "#5e81ac"
Set BorderRadius 10

Set TypingSpeed 80ms

Set Framerate 60

# --- Background setup ---
Hide
Type "export EDITOR=vim"
Enter
Sleep 300ms
Type "tmux new-session -d -s start"
Enter
Sleep 300ms
Type "tmux new-session -d -s to-kill"
Enter
Sleep 300ms
Type "tmux new-session -d -s to-rename"
Enter
Sleep 300ms
Type "tsman save start"
Enter
Sleep 500ms
Type "tsman save to-kill"
Enter
Sleep 500ms
Type "tsman save to-rename"
Enter
Sleep 500ms
Type "tmux attach -t start"
Enter
Sleep 500ms
Show

Ctrl+L
Sleep 1s

# Scene 1: open the menu with confirmation prompts
Type "tsman menu -a -p"
Sleep 1s
Enter
Sleep 1s

# Scene 2: navigation
Down
Sleep 500ms
Down
Sleep 500ms
Down
Sleep 500ms
Down
Sleep 500ms
Down
Sleep 500ms
Up
Sleep 500ms

# Scene 3: help popup
Ctrl+H
Sleep 2s
Escape
Sleep 1s

# Scene 4: fuzzy search
Type "kill"
Sleep 1s
Ctrl+U
Sleep 1s

# Scene 5: edit a session config
Ctrl+E
Sleep 2s
Type ":q!"
Enter
Sleep 2s

# Scene 6: kill an active session
Type "kill"
Sleep 1s
Ctrl+K
Sleep 1s

# Scene 7: rename then delete a session
Ctrl+U
Sleep 1s

Type "rename"
Sleep 1s
Ctrl+R
Sleep 1s

Ctrl+U
Type "vhs-demo-session"
Sleep 1s
Enter
Sleep 1s

Type "start"
Ctrl+D
Sleep 1s
Type "y"
Sleep 1s

# Scene 8: toggle preview
Ctrl+T
Sleep 1s
Ctrl+T
Sleep 1s

# Scene 9: switch to layouts view
Ctrl+L
Sleep 2s

# Scene 10: create a session from a layout
Enter
Sleep 1s
Type "my-project"
Enter
Sleep 1s
Type "/home/tecuc/Dev"
Enter
Sleep 1s

# Scene 11: search for newly created layout
Type "tsman menu -a -p"
Sleep 1s
Enter
Sleep 1s

Type "prjct"
Sleep 3s

# Scene 12: exit
Escape
Sleep 1s

# --- Cleanup: detach ---
Hide
Type "tmux detach"
Enter
Show
