# cli-demo.tape — VHS tape for MultiVC CLI demo GIF
#
# Usage:
#   vhs docs/cli-demo.tape
#
# Requirements:
#   - vhs (https://github.com/charmbracelet/vhs)
#   - pre-built: cargo build -p cli --release

Output docs/assets/cli-demo.gif

# Terminal settings
Set Shell "cmd"
Set FontFamily "JetBrains Mono"
Set FontSize 16
Set Width 1200
Set Height 1000
Set Padding 20
Set Theme "Catppuccin Mocha"
Set TypingSpeed 50ms
Set Framerate 30
Set WindowBar Colorful
Set Margin 20
Set MarginFill "#1e1e2e"
Set BorderRadius 10

# Hidden setup
Hide
Type "cls"
Enter
Sleep 500ms
Show

# 1. Help — overview of available commands
Type "target\release\cli.exe --help"
Enter
Sleep 3s

# 2. Fetch — list available versions from GitHub
Type "target\release\cli.exe fetch"
Enter
Sleep 3s

# 3. Install — download and install a core version
Type "target\release\cli.exe install 0.31.1"
Enter
Sleep 3s

# 4. List — show installed cores
Type "target\release\cli.exe ls"
Enter
Sleep 2s

# 5. Create instance
Type "target\release\cli.exe new demo 0.31.1"
Enter
Sleep 2s

# 6. List instances
Type "target\release\cli.exe instances"
Enter
Sleep 2s

# 7. Launch instance (stop after 3s)
Type "target\release\cli.exe launch demo"
Enter
Sleep 3s
Ctrl+C
Sleep 2s

# 8. Validate integrity
Type "target\release\cli.exe check"
Enter
Sleep 3s

# 9. Remove instance
Type "target\release\cli.exe rmi demo"
Enter
Sleep 2s

# 10. Remove core
Type "target\release\cli.exe rm 0.31.1"
Enter
Sleep 2s
