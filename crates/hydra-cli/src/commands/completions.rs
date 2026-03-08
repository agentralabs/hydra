use crate::output;

pub fn generate(shell: &str) {
    match shell {
        "bash" => println!("{}", generate_bash()),
        "zsh" => println!("{}", generate_zsh()),
        "fish" => println!("{}", generate_fish()),
        _ => {
            output::print_error(&format!("Unknown shell: {}", shell));
            output::print_info("Supported shells: bash, zsh, fish");
        }
    }
}

pub fn generate_bash() -> String {
    r#"# Hydra bash completions
# Add to ~/.bashrc: eval "$(hydra completions bash)"

_hydra_completions() {
    local cur prev commands
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    commands="run status approve deny freeze resume kill inspect config sisters skills completions help version"

    case "${prev}" in
        hydra)
            COMPREPLY=( $(compgen -W "${commands}" -- "${cur}") )
            return 0
            ;;
        config)
            COMPREPLY=( $(compgen -W "show set get" -- "${cur}") )
            return 0
            ;;
        sisters)
            COMPREPLY=( $(compgen -W "status connect disconnect" -- "${cur}") )
            return 0
            ;;
        skills)
            COMPREPLY=( $(compgen -W "list install remove search" -- "${cur}") )
            return 0
            ;;
        completions)
            COMPREPLY=( $(compgen -W "bash zsh fish" -- "${cur}") )
            return 0
            ;;
        connect|disconnect)
            local sisters="memory vision codebase identity time contract comm planning cognition reality forge aegis veritas evolve"
            COMPREPLY=( $(compgen -W "${sisters}" -- "${cur}") )
            return 0
            ;;
        run)
            COMPREPLY=( $(compgen -W "--auto-approve --dry-run --verbose --quiet --timeout" -- "${cur}") )
            return 0
            ;;
        inspect)
            COMPREPLY=( $(compgen -W "--format" -- "${cur}") )
            return 0
            ;;
        --format)
            COMPREPLY=( $(compgen -W "text json yaml" -- "${cur}") )
            return 0
            ;;
    esac
}
complete -F _hydra_completions hydra"#
        .to_string()
}

pub fn generate_zsh() -> String {
    r#"# Hydra zsh completions
# Add to ~/.zshrc: eval "$(hydra completions zsh)"

_hydra() {
    local -a commands subcommands sisters
    commands=(
        'run:Execute an intent'
        'status:Show run status'
        'approve:Approve a pending action'
        'deny:Deny a pending action'
        'freeze:Freeze active runs'
        'resume:Resume a frozen run'
        'kill:Kill active runs'
        'inspect:Inspect a run in detail'
        'config:Manage configuration'
        'sisters:Manage sister connections'
        'skills:Manage skills'
        'completions:Generate shell completions'
        'help:Show help'
        'version:Show version'
    )
    sisters=(memory vision codebase identity time contract comm planning cognition reality forge aegis veritas evolve)

    _arguments -C \
        '1:command:->command' \
        '*::arg:->args'

    case $state in
        command)
            _describe 'hydra command' commands
            ;;
        args)
            case $words[1] in
                config)
                    subcommands=('show:Show all config' 'set:Set a config value' 'get:Get a config value')
                    _describe 'config subcommand' subcommands
                    ;;
                sisters)
                    subcommands=('status:Show sister status' 'connect:Connect a sister' 'disconnect:Disconnect a sister')
                    _describe 'sisters subcommand' subcommands
                    ;;
                skills)
                    subcommands=('list:List installed skills' 'install:Install a skill' 'remove:Remove a skill' 'search:Search for skills')
                    _describe 'skills subcommand' subcommands
                    ;;
                completions)
                    subcommands=('bash:Bash completions' 'zsh:Zsh completions' 'fish:Fish completions')
                    _describe 'shell' subcommands
                    ;;
                connect|disconnect)
                    _describe 'sister' sisters
                    ;;
                run)
                    _arguments \
                        '--auto-approve[Auto-approve all actions]' \
                        '--dry-run[Show plan without executing]' \
                        '--verbose[Show detailed output]' \
                        '--quiet[Minimal output]' \
                        '--timeout[Set timeout in seconds]:seconds:'
                    ;;
                inspect)
                    _arguments '--format[Output format]:format:(text json yaml)'
                    ;;
            esac
            ;;
    esac
}
compdef _hydra hydra"#
        .to_string()
}

pub fn generate_fish() -> String {
    r#"# Hydra fish completions
# Add to ~/.config/fish/completions/hydra.fish

# Main commands
complete -c hydra -n __fish_use_subcommand -a run -d 'Execute an intent'
complete -c hydra -n __fish_use_subcommand -a status -d 'Show run status'
complete -c hydra -n __fish_use_subcommand -a approve -d 'Approve a pending action'
complete -c hydra -n __fish_use_subcommand -a deny -d 'Deny a pending action'
complete -c hydra -n __fish_use_subcommand -a freeze -d 'Freeze active runs'
complete -c hydra -n __fish_use_subcommand -a resume -d 'Resume a frozen run'
complete -c hydra -n __fish_use_subcommand -a kill -d 'Kill active runs'
complete -c hydra -n __fish_use_subcommand -a inspect -d 'Inspect a run in detail'
complete -c hydra -n __fish_use_subcommand -a config -d 'Manage configuration'
complete -c hydra -n __fish_use_subcommand -a sisters -d 'Manage sister connections'
complete -c hydra -n __fish_use_subcommand -a skills -d 'Manage skills'
complete -c hydra -n __fish_use_subcommand -a completions -d 'Generate shell completions'
complete -c hydra -n __fish_use_subcommand -a help -d 'Show help'
complete -c hydra -n __fish_use_subcommand -a version -d 'Show version'

# config subcommands
complete -c hydra -n '__fish_seen_subcommand_from config' -a show -d 'Show all config'
complete -c hydra -n '__fish_seen_subcommand_from config' -a set -d 'Set a config value'
complete -c hydra -n '__fish_seen_subcommand_from config' -a get -d 'Get a config value'

# sisters subcommands
complete -c hydra -n '__fish_seen_subcommand_from sisters' -a status -d 'Show sister status'
complete -c hydra -n '__fish_seen_subcommand_from sisters' -a connect -d 'Connect a sister'
complete -c hydra -n '__fish_seen_subcommand_from sisters' -a disconnect -d 'Disconnect a sister'

# Sister names for connect/disconnect
set -l sisters memory vision codebase identity time contract comm planning cognition reality forge aegis veritas evolve
complete -c hydra -n '__fish_seen_subcommand_from connect' -a "$sisters"
complete -c hydra -n '__fish_seen_subcommand_from disconnect' -a "$sisters"

# skills subcommands
complete -c hydra -n '__fish_seen_subcommand_from skills' -a list -d 'List installed skills'
complete -c hydra -n '__fish_seen_subcommand_from skills' -a install -d 'Install a skill'
complete -c hydra -n '__fish_seen_subcommand_from skills' -a remove -d 'Remove a skill'
complete -c hydra -n '__fish_seen_subcommand_from skills' -a search -d 'Search for skills'

# completions subcommands
complete -c hydra -n '__fish_seen_subcommand_from completions' -a 'bash zsh fish'

# run flags
complete -c hydra -n '__fish_seen_subcommand_from run' -l auto-approve -d 'Auto-approve all actions'
complete -c hydra -n '__fish_seen_subcommand_from run' -l dry-run -d 'Show plan without executing'
complete -c hydra -n '__fish_seen_subcommand_from run' -l verbose -d 'Show detailed output'
complete -c hydra -n '__fish_seen_subcommand_from run' -l quiet -d 'Minimal output'
complete -c hydra -n '__fish_seen_subcommand_from run' -l timeout -d 'Set timeout in seconds'

# inspect flags
complete -c hydra -n '__fish_seen_subcommand_from inspect' -l format -a 'text json yaml' -d 'Output format'"#
        .to_string()
}
