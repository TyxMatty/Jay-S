<p align="center"><img src="frontend/src-tauri/icons/128x128@2x.png" alt="The Jay-S wolf: a hand-drawn wolf head in orange lines" width="112"></p>

<h1 align="center">Jay-S</h1>

<p align="center"><strong>Study with real exams and your own material, next to an AI that runs on your computer.</strong></p>

<p align="center">
  <a href="https://github.com/TyxMatty/Jay-S/actions/workflows/ci.yml"><img src="https://github.com/TyxMatty/Jay-S/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
</p>

<p align="center"><strong>English</strong> · <a href="README.pt-BR.md">Português</a></p>

<p align="center"><img src="docs/images/hero.png" alt="Jay-S on the Ready-made exams tab: the UEPG Vestibular 2025 exams, 45 sum-type questions each with the official answer key; Grupo 1 and Grupo 2 are downloaded and ready to take, the others have a Download button" width="880"></p>

Jay-S is the study app of the Jay line, for Windows. Take a real entrance exam, or attach your handouts and exams and let **Jay-AI** build exams in the style you pick (ENEM, UEPG, UEM, UNICENTRO…). Answer them in the app, in exam mode or in a learning mode that shows each answer at once, or against the clock as a mock test; then keep going with explanations and flashcards. Jay-AI is a language model that runs on your own computer: your files and answers never leave it.

## Ready-made exams

The UEPG Vestibular 2025 exams come ready to take, each with its **official answer key**. Press Download and Jay-S fetches the PDF from the university's website, checks it and splits it into questions. Nothing of yours is sent.

## Every question with its text, and the original page

Questions keep their base text, and **See in the original exam** opens the real PDF pages the question is on, for when a formula, table or figure didn't survive the conversion to text.

<table>
  <tr>
    <td><img src="docs/images/question.png" alt="Question 31 of 45 of UEPG 2025 Grupo 1, about slavery in Greece and Rome: its base text in a highlighted box, the statement and five sum-type statements, two of them marked"></td>
    <td><img src="docs/images/original.png" alt="The same question with the original exam open: the real PDF page, with question 31 under the HISTÓRIA heading and question 33 beside it"></td>
  </tr>
</table>

## Jay-AI looks at your result

When you finish, Jay-S grades the exam with the key, question by question (sum-type questions get partial credit), and sends the result to Jay-AI, which writes what went well, what to review and what to do next.

<p align="center"><img src="docs/images/result.png" alt="The result of a UEPG 2025 Grupo 1 attempt: 33.1 of 45, 74%, and Jay-AI's analysis below it, written in Portuguese, with a summary and the sections What went well and What to review" width="880"></p>

The analysis is the model's reading of your answers, not a grade: like any language model, it can get details wrong.

## A chat that computes

Ask Jay-AI about anything, about your materials, or about an exam you took. When the question is a calculation, Jay-AI picks the formula and **Jay-S does the arithmetic**: every number is computed by the app, and for a hypothesis test the decision (reject H0 or not) is the app's comparison of the statistic with the critical value. The reply says so with a "Computed by the app" badge, and asks you to check the formula against your material. (In the picture, Jay-AI wrote the steps in Portuguese.)

<p align="center"><img src="docs/images/chat.png" alt="A Jay-AI conversation about a statistics handout: the question gives two samples (n = 40, mean 72.5, standard deviation 8; n = 50, mean 68.9, standard deviation 9) and asks whether to reject H0 at the 5% level; the reply, marked Computed by the app, says to reject H0 because |2.0062| is at least the critical value 1.96, and lists the steps, in English like the question, with the standard error 1.7944 and Z = 2.0062" width="792"></p>

## Explanations and flashcards

**Explain** turns a handout into a short text, key points and concept cards, in the language and depth you choose, then asks you questions to see what stuck. Flashcards come from your material and space themselves out as you rate them; a rating can be undone, and each session ends with a summary.

<table>
  <tr>
    <td><img src="docs/images/explain.png" alt="Jay-AI's explanation of a one-page handout on the Z test for two means, in English at the Balanced level: a title and five paragraphs on hypotheses, the statistic, critical values and what not rejecting H0 means"></td>
    <td><img src="docs/images/flashcards.png" alt="A flashcard under review in the deck Comparing two means, 8 cards left: the answer side shows the Z formula for two independent means, with the buttons Forgot (10 minutes), Hard (12 hours), Good (1 day) and Easy (4 days)"></td>
  </tr>
</table>

## Three styles, light and dark

Calm, Neon and Soft, each in a light and a dark theme.

<table>
  <tr>
    <td><img src="docs/images/style-calm.png" alt="The key points and concept cards of the Z test explanation in the Calm style: dark background with purple accents and a purple wolf"></td>
    <td><img src="docs/images/style-neon.png" alt="The same screen in the Neon style: near-black background with cyan accents and a cyan wolf"></td>
    <td><img src="docs/images/style-soft.png" alt="The same screen in the Soft style: warm dark background with orange accents and an orange wolf"></td>
  </tr>
  <tr align="center"><td>Calm</td><td>Neon</td><td>Soft</td></tr>
</table>

<p align="center"><img src="docs/images/light.png" alt="The Ready-made exams tab in the light theme with the Soft style: white cards and orange buttons, Grupo 1 and Grupo 2 ready to take" width="880"></p>


## Private by design

Everything runs on your computer: the app, Jay-AI and your data. Jay-S uses the network for three things only:

- **Jay-AI's download**, once, when you turn it on: its runtime and the model (1.3 or 2.7 GB).
- **A ready-made exam**, when you press its Download button, from the university's website.
- **The update check**, once per start, against this repository's releases; an update installs only when you click it.

Text from your files and from the model is shown as text, never as page code, and the page never handles file paths. The security review is in [docs/SEGURANCA.en.md](docs/SEGURANCA.en.md).

It is also light: with Jay-AI idle, the release build measured about 110 to 120 MB of private memory, and Jay-AI leaves the memory 2 minutes after its last use.

## Download

Download `Jay-S_<version>_x64-setup.exe` from the **[latest release](https://github.com/TyxMatty/Jay-S/releases/latest)** (Windows, 64-bit). Jay-AI is not in the installer: you download it in the app, in the Jay-AI section. Every version is listed under [releases](https://github.com/TyxMatty/Jay-S/releases).

## Code

Tauri v2 with vanilla JavaScript for the interface (`frontend/`) and a Rust engine (`crates/jay-study`); Jay-AI is `llama-server` from llama.cpp, run as a child process. The plan, the stages and what was measured are in [docs/ARQUITETURA.en.md](docs/ARQUITETURA.en.md).

```powershell
npm install
npm run dev    # the app (Tauri), Vite on port 5178
npm run web    # just the interface, in a browser
npm test
```

License: MIT.
