Source common.tape
Output compact.mp4
Env SCENE_FILE "swap/Swap.lean:4"
Source open.tape

Ctrl+B Right Sleep 500ms
Type "[" Sleep 1s
Type "[" Sleep 2s
Ctrl+B Left Sleep 500ms
Type ":5" Enter Type "gs" Sleep 300ms Type " h" Sleep 2500ms Type ",;"
Type ":7" Enter Type "gs" Sleep 300ms Type " h" Sleep 2500ms Type ",;"
Type ":10" Enter Type "gs" Sleep 300ms Type " h" Sleep 4s
