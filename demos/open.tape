Hide
Type "tmux -f tmux.conf new-session -d -s demo hx $SCENE_FILE" Enter
Type "tmux -f tmux.conf split-window -h -t demo lean-tui" Enter
Type "tmux -f tmux.conf select-pane -t demo:0.0" Enter
Type "tmux -f tmux.conf attach -t demo" Enter
Wait+Screen /NOR /
Type "gs" Type " h"
Wait+Screen /\d+:\d+ \(\S+\)/
Type ",;"
Sleep 1s
Show
