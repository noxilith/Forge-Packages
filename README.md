# Forge recipes

A [Forge](https://github.com/noxilith/Forge) tap: a git repository of build recipes for packages that
need dependencies on other Forge packages or special isolation settings.
Small, self-contained projects (fastfetch and the like) don't belong here;
`forge install <git-url>` handles those on its own with auto-detection.

```
forge tap add <url-of-this-repository>
forge install niri                                # by name
forge install https://github.com/YaLTeR/niri     # by URL: uses the recipe here
forge install --ignore-taps <url>                 # ignore recipes, auto-detect
forge tap update                                  # fetch newer recipes
```

Each recipe is `recipes/<name>.toml`. Recipes are build instructions that run
in Forge's sandbox, but they are still code you are choosing to run, so only
add taps you trust.

`recipes/hyprland.toml` and `recipes/hyprcursor.toml` are marked unverified in
their comments: the dependency chain they sit on (hyprutils, hyprlang,
hyprwayland-scanner, aquamarine) was built and tested, but the machine used to
write them lacked some host libraries needed to build those two.

## License

MIT. See [LICENSE](LICENSE).
