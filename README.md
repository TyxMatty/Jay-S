# Jay-S

[![CI](https://github.com/TyxMatty/Jay-S/actions/workflows/ci.yml/badge.svg)](https://github.com/TyxMatty/Jay-S/actions/workflows/ci.yml)

**English** · [Português](README.pt-BR.md)

The study app of the Jay line: attach your content and your exams, and **Jay-AI**, a model that runs on your computer, builds exams in the style you pick (ENEM, UEPG, UEM, UNICENTRO…) that you answer right in the app (multiple choice or sum-type questions, with a learning mode that shows each answer at once), timed mock tests and flashcards that adapt to you. Your files never leave your computer.

<p align="center"><img src="docs/images/explain.png" alt="Jay-S explaining a handout about photosynthesis: a list of key points and cards with the concepts, written by Jay-AI" width="820"></p>

<table>
  <tr>
    <td><img src="docs/images/chat-calm.png" alt="A new conversation with Jay-AI in the Calm style, purple"></td>
    <td><img src="docs/images/chat-neon.png" alt="A new conversation with Jay-AI in the Neon style, cyan"></td>
    <td><img src="docs/images/chat-soft.png" alt="A new conversation with Jay-AI in the Soft style, orange"></td>
  </tr>
  <tr align="center"><td>Calm</td><td>Neon</td><td>Soft</td></tr>
</table>

## Download

Download `Jay-S_<version>_x64-setup.exe` from the **[latest release](https://github.com/TyxMatty/Jay-S/releases/latest)** (Windows, 64-bit). Jay-AI is not in the installer: its runtime and model (1.3 or 2.7 GB) are downloaded once, when you turn Jay-AI on in the app. Every version is listed under [releases](https://github.com/TyxMatty/Jay-S/releases).

The plan, the stages and what was measured are in [docs/ARQUITETURA.en.md](docs/ARQUITETURA.en.md). The security review (trust boundaries, CWE and OWASP LLM mapping, Semgrep, open findings) is in [docs/SEGURANCA.en.md](docs/SEGURANCA.en.md).

## Code

```powershell
npm install
npm run dev    # the app (Tauri), Vite on port 5178
npm run web    # just the interface, in a browser
npm test
```

License: MIT.
