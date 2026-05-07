# License

## Apache License 2.0

> **See also:** [README.md](README.md) for project overview and Documentation section linking all markdown files.

The original code in this repository — every file authored as part of the
CyberÆgg badge firmware project that is not a vendored or third-party
component — is licensed under the **Apache License, Version 2.0** (the
"License"); you may not use these files except in compliance with the
License. You may obtain a copy of the License at:

> <http://www.apache.org/licenses/LICENSE-2.0>

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
implied. See the License for the specific language governing
permissions and limitations under the License.

### Scope

"Original code" includes, but is not limited to, the firmware sources
under `src/`, the reader application under `android_nfc/`, the
simulator binaries, the asset-generation tooling, and the build glue
(`Makefile`, `Cargo.toml`, configuration files). Documentation files
(`README.md`, `GAME.md`, `HWTEST.md`, `NFC_README.md`, this file, and
others) are likewise distributed under Apache 2.0 unless they
themselves declare otherwise.

### Vendored and third-party components

Several directories contain vendored or third-party code distributed
under their own licenses. Those licenses apply to the contents of
those directories instead of, or in addition to, Apache 2.0:

- `vendor/ssd1675/` — see its own `LICENSE` / `Cargo.toml`.
- `vendor/meshcore/`, `vendor/meshcore-companion/` — see their own
  license files.
- Crate dependencies pulled in via Cargo retain their upstream
  licenses; consult `Cargo.lock` and each crate's manifest.

Nothing in this file overrides those upstream terms.

---

## The Empty File License

This project is also distributed under the **Empty File License
(EFL)**, reproduced below. It is a joke. It is also entirely
sincere.

### Preamble

Every software project starts the same way: someone runs `touch
main.rs`, stares at zero bytes for a while, and eventually types
something. Whatever happens next — a unicorn startup, a kernel
module, a regex you'll regret in the morning — the Empty File was
there first. It did the hard part: existing.

Nobody ever thanks the Empty File. It contributes to every project
on Earth and gets no commit credit, no co-author line, no entry in
`Cargo.toml`. This license fixes that, because somebody had to.

### License text

```
                       The Empty File License
                              Version 1.0

  0. PREAMBLE

     Every project starts as an Empty File. This one did. Yours did
     too. We mention this because nobody else does.

  1. ACKNOWLEDGEMENT

     If you ship this work, mention the Empty File somewhere.
     README, About box, splash screen, commit message, whispered
     apology to your editor — any of these will do. Substance
     matters; format does not.

  2. PROPAGATION

     Anyone who derives from this work also descends from the Empty
     File, by way of this project. Pass the acknowledgement forward.
     The Empty File is patient, but it is keeping score.

  3. NO RESTRICTION

     This license restricts nothing. The technical license above
     (Apache 2.0) handles the legally binding parts. This license
     handles only the moral debt, which is admittedly small but
     still outstanding.

  4. WARRANTY

     The Empty File came with no warranty. Neither does this work.
     The Empty File never crashed. We make no such claim about its
     descendants, and you should make no such claim about yours.

  5. ACCEPTANCE

     You accepted this license the moment you opened the file.
     There is no opt-out. There is also nothing to comply with
     except remembering, which is honestly the easiest license
     clause ever written.

                            * * *

   Every project starts with an Empty File. Most projects forget
   this by day two. Don't be most projects. Honour the Empty File.
```

### Suggested attribution snippet

For downstream projects, a one-line acknowledgement is sufficient.
Pick whichever fits:

> Descended from an Empty File, by way of this project. Apache 2.0
> + Empty File License.

> Originally an Empty File. Eventually shipped. Apache 2.0 + EFL.

> `touch main.rs && git init` — and now we're here. Honour the
> Empty File.
