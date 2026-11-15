# Contributing to skyglow

Thanks for being here. You don't need to write Rust to help. Some of the most valuable contributions are data.

## Ways to help

1. **Share an observation.** Real timed notes from a sunset are the most useful thing you can give, because the colour model is calibrated on them. See [docs/OBSERVATIONS.md](docs/OBSERVATIONS.md).
2. **Add a place** as a new file `examples/places/<name>.json` (see docs/PLACES.md) (a city, not a home address, see privacy below).
3. **Improve a regional pack** in `packs/` (cloud heights, stage boundaries, colour hints). Say in the pack's `description` why you changed the numbers: photos, soundings, a paper.
4. **Improve the docs.** If anything in [docs/HOW-IT-WORKS.md](docs/HOW-IT-WORKS.md) confused you, that is a bug. Tell us or fix it.
5. **Code.** Rust for the engine in `core/`; later Vue/TypeScript for `web/` and Python for `lab/`.

## A note on privacy (recommendations)

Sunset photos usually come from where you live, so you may not want to publish your exact spot. It is your choice, and here is what we suggest:

- For public observations, consider using the nearest city or coordinates rounded to one decimal place (about 11 km). That is plenty for the model.
- Keep exact coordinates in your own `test/places/` folder, which is git-ignored.
- Glance at `git status` before you commit to confirm nothing from `test/` is staged.
- Times and short notes are enough; there is no need to commit raw photos or full-resolution EXIF.

## Before you open a pull request

```powershell
cargo test            # everything should pass
cargo fmt             # standard Rust formatting
git status            # confirm no private files are staged
```

If you added a place, a pack or an observation example, `cargo test` already checks that it is valid (see [docs/TESTING.md](docs/TESTING.md)).

## Style

- Explain the *why*, in plain words, in comments and docs. Someone should be able to learn the physics from the code.
- Prefer small, testable functions with no hidden state.
- Tests need an independent source for the expected value (a known fact or a hand calculation), not a copy of the code's output.

## Licence

By contributing you agree that your contribution is licensed under the MIT licence of this project.
