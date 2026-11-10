# Jay-S — contexto para o Claude

App de estudos da linha Jay (Tauri v2 + Vanilla JS): provas, simulados e flashcards gerados a partir do material do estudante por um modelo local (o Jay-AI, `llama-server` do llama.cpp num processo filho).
- **Arquitetura, decisões, segurança e etapas:** [docs/ARQUITETURA.md](docs/ARQUITETURA.md). A tabela da seção 8 marca o que está feito (✅).
- **Base:** copiada do Jay-H (estilos, tema, idioma, ícone da janela) e do Jay-A (navegação lateral, cards). Lições e armadilhas da linha estão no `CLAUDE.md` do Jay-C (`../JayC/JayC/CLAUDE.md`), seção "Fatos técnicos".

## Como trabalhar com o usuário

- **Idioma:** responda no idioma da última mensagem dele. Docs e commits em português; código e comentários em inglês.
- **Commits:** **nunca** inclua `Co-Authored-By: Claude` nem outra atribuição. Assunto + corpo com o quê, por quê e o que foi verificado.
- **Ritmo:** uma etapa por vez, cada uma terminando em algo testável. Ao fim: commit, resumo e pergunta se segue.
- **Verificação:** prove com medição. Se um teste não prova nada, diga.
- **Segurança:** toda mudança que possa pôr dados do estudante em risco (anexos, pasta de dados, saída do modelo, `llama-server`, downloads, IPC) passa pelos 10 pontos da seção 7 da arquitetura, mesmo sendo tudo local. Registre o resultado na seção 7 e escreva os testes adversariais.
- **README e docs:** o `README.md` é em inglês (com o `README.pt-BR.md` ao lado), e tudo o que ele linka precisa ter versão em inglês (`*.en.md`). Mudou um doc em português? Atualize o `.en.md` junto.
- **Pouca RAM:** é requisito (RNF-1). Meça a memória da árvore do app antes de dizer que algo é leve.

## Build e execução (Windows)

```bash
npm install
npm run dev          # app Tauri, Vite em :5178
npm run web          # só a interface no navegador
npm test             # node:test
```

- **Rust fora do OneDrive:** `.cargo/config.toml` (local, fora do git) aponta o `target-dir` para `C:/Users/<você>/.cargo-target/jay-s`.
- **Mudou o `crates/jay-study`?** O `tauri dev` só vigia o `src-tauri`: o app continua com o motor velho. Mude a data de um arquivo do `src-tauri` (`touch src/main.rs`) ou reinicie o `npm run dev`.
- **Testar o app sem clicar:** `$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS='--remote-debugging-port=9227'; npm run dev`, depois `node scripts/cdp-eval.mjs scripts/test-materials.js` (roda JS dentro do app) e `node scripts/cdp-shot.mjs saida.png` (print só da página do app). Anexar sem o diálogo: um segundo `jay-s.exe arquivo.pdf` (a instância única entrega os arquivos).
- **Jay-AI num PC de dev:** `cargo run --release --example ai_setup -- "$env:APPDATA\io.github.tyxmatty.jays"` baixa o runtime e o modelo como o app; `JAYS_AI_ROOT=<mesma pasta> cargo test -- --ignored server` testa o servidor de verdade. Para medir a RAM do servidor, olhe o *working set*: o "privado" inclui a memória da GPU.
- **Treino do classificador (etapa 8):** o Python fica num ambiente virtual fora do repositório e do OneDrive (`C:/Users/<você>/.venvs/jay-s-ml`, com `pip install --only-binary=:all: -r ml/requirements.txt`); `<venv>/Scripts/python -I ml/train.py > ml/results.txt` refaz as medidas a partir do `ml/signals.csv` (3,7 s, mesma saída a cada rodada). O `npm test` não roda Python: mudou o `signals.csv`, refaça o `results.txt` à mão.
- **Nunca rode `cargo` em paralelo com o `tauri dev` no mesmo `target-dir`:** os dois compilaram o `jay-study` ao mesmo tempo e o link quebrou (`LNK2019` em símbolos `anon.*.llvm.*`); só um `cargo clean -p jay-study` resolveu. Para os testes do motor com o app aberto: `CARGO_TARGET_DIR=C:/Users/<você>/.cargo-target/jay-s-engine cargo test`.
- **Nunca edite arquivo com `Get-Content -Raw` no PowerShell 5.1:** ele lê como ANSI e grava os acentos estragados (`â€¦` no lugar de `…`); o `npm test` pega no `pt-BR.js`, mas não no Rust. Edite com a ferramenta de edição ou com Python/Node em UTF-8.
- **Python da Microsoft Store não vê o `%APPDATA%` de verdade** (ele redireciona para uma pasta própria): para ler a pasta de dados do Jay-S, use Node ou PowerShell.
- **WebView2 sem GPU:** a janela abre com `--disable-gpu` (`tauri.conf.json`), o que tira ~80 MB de RAM (RNF-1). Ao rodar com `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` (depuração), essa variável vale no lugar dos argumentos do config: para medir RAM, rode sem ela.
- **Processos iniciados pelo app de desktop do Claude veem um AppData privado:** ele é um pacote MSIX, e tudo o que inicia (`npm run dev`, um instalador, o `jay-s.exe`) grava `%APPDATA%` e `%LOCALAPPDATA%` em `%LOCALAPPDATA%\Packages\Claude_pzs8sxrjxfjjc\LocalCache\{Roaming,Local}`; o Explorer e os apps que o usuário abre não veem esses arquivos. `%TEMP%`, a raiz do perfil, a Área de Trabalho e o menu Iniciar são os de verdade. Então: a pasta de dados do Jay-S de dev (com o modelo) fica nesse `LocalCache\Roaming\io.github.tyxmatty.jays`, e um Jay-S aberto pelo usuário começa com outra, vazia; um instalador rodado daqui não instala de verdade (e deixa atalhos de verdade apontando para o nada); instalação, ícone da barra de tarefas e Jay-H só se provam com o usuário rodando por fora.
- **Ícone da barra de tarefas:** o app ID e o "relaunch" só valem para o app instalado (`%LOCALAPPDATA%\Jay-S`), onde desamarram a janela do atalho do menu Iniciar; fora dele, só o `ICON_BIG`. O Explorer carrega o .ico sozinho: uma página em branco no botão quer dizer que o arquivo não existe para ele (item acima), não que o ícone esteja errado. E ele guarda o que desenhou primeiro para um app ID, a página em branco inclusive: por isso o ID da janela leva o hash da figura (`taskbar_icon.rs`); limpar o ID e pôr o mesmo de volta não adianta. Para ver o botão: `powershell -File scripts/taskbar-button.ps1 saida.png` (recorta só o botão do Jay-S, nada mais da tela).
- **Espaço na interface:** escala de 4/8 px (`--space-*` no `app.css`), cartões com 24 px de respiro em todos os lados; confira com print cada tela mudada, procurando partes apertadas ou sobrepostas.
- **Portas:** Vite 5178, depuração remota 9227 (Jay-C 5173/9222, Jay-A 5175, Jay-H 5176, Jay-E 5177/9226).
- **Regras que vieram da linha:** `[hidden] { display: none !important }` global; texto de arquivo ou do modelo só por `textContent` (o `h()` de `ui/dom.js`), nunca `innerHTML`; a página nunca manda caminho de arquivo; dados do estudante em arquivos do Rust (o `localStorage` do WebView2 vai ao disco com atraso e se perde se o app for morto).
