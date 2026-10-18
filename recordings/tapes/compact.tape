Source common.tape
Output compact.gif
Env SCENE_FILE "Samples/swap_tactic.lean:6"
Source open.tape

Ctrl+B Right Sleep 500ms
Type "[" Sleep 1s
Type "[" Sleep 2s
Ctrl+B Left Sleep 500ms
Type ":7" Enter Type "gs" Sleep 300ms Type " k" Sleep 1s Escape Sleep 1500ms
Type ":9" Enter Type "gs" Sleep 300ms Type " k" Sleep 1s Escape Sleep 1500ms
Type ":12" Enter Type "gs" Sleep 300ms Type " k" Sleep 1500ms Escape Sleep 3s
