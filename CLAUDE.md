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
- **Nunca rode `cargo` em paralelo com o `tauri dev` no mesmo `target-dir`:** os dois compilaram o `jay-study` ao mesmo tempo e o link quebrou (`LNK2019` em símbolos `anon.*.llvm.*`); só um `cargo clean -p jay-study` resolveu. Para os testes do motor com o app aberto: `CARGO_TARGET_DIR=C:/Users/<você>/.cargo-target/jay-s-engine cargo test`.
- **Python da Microsoft Store não vê o `%APPDATA%` de verdade** (ele redireciona para uma pasta própria): para ler a pasta de dados do Jay-S, use Node ou PowerShell.
- **Portas:** Vite 5178, depuração remota 9227 (Jay-C 5173/9222, Jay-A 5175, Jay-H 5176, Jay-E 5177/9226).
- **Regras que vieram da linha:** `[hidden] { display: none !important }` global; texto de arquivo ou do modelo só por `textContent` (o `h()` de `ui/dom.js`), nunca `innerHTML`; a página nunca manda caminho de arquivo; dados do estudante em arquivos do Rust (o `localStorage` do WebView2 vai ao disco com atraso e se perde se o app for morto).
