<p align="center"><img src="frontend/src-tauri/icons/128x128@2x.png" alt="O lobo do Jay-S: uma cabeça de lobo desenhada à mão, em traço laranja" width="112"></p>

<h1 align="center">Jay-S</h1>

<p align="center"><strong>Estude com provas de verdade e com o seu material, ao lado de uma IA que roda no seu computador.</strong></p>

<p align="center">
  <a href="https://github.com/TyxMatty/Jay-S/actions/workflows/ci.yml"><img src="https://github.com/TyxMatty/Jay-S/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
</p>

<p align="center"><a href="README.md">English</a> · <strong>Português</strong></p>

<p align="center"><img src="docs/images/hero.png" alt="O Jay-S na aba Provas prontas (com a interface em inglês): as provas do Vestibular UEPG 2025, de 45 questões de somatória cada, com o gabarito oficial; o Grupo 1 e o Grupo 2 já estão baixados e prontos para fazer, os outros têm o botão de baixar" width="880"></p>

O Jay-S é o app de estudos da linha Jay, para Windows. Faça uma prova de vestibular de verdade, ou anexe suas apostilas e provas e deixe o **Jay-AI** montar provas no estilo que você escolher (ENEM, UEPG, UEM, UNICENTRO…). Responda no próprio app, no modo prova ou num modo aprender que mostra cada resposta na hora, ou contra o relógio, como simulado; depois continue com explicações e flashcards. O Jay-AI é um modelo de linguagem que roda no seu computador: seus arquivos e respostas não saem dele.

## Provas prontas

As provas do Vestibular UEPG 2025 vêm prontas para fazer, cada uma com o **gabarito oficial**. Clique em baixar e o Jay-S busca o PDF no site da universidade, confere o arquivo e separa as questões. Nada seu é enviado.

## Cada questão com o seu texto, e a página original

As questões mantêm o texto-base, e **Ver na prova original** abre as páginas do PDF em que a questão está, para quando uma fórmula, tabela ou figura não sobreviveu à conversão em texto.

<table>
  <tr>
    <td><img src="docs/images/question.png" alt="Questão 31 de 45 da UEPG 2025 Grupo 1, sobre a escravidão na Grécia e em Roma: o texto-base num quadro destacado, o enunciado e cinco proposições de somatória, duas delas marcadas"></td>
    <td><img src="docs/images/original.png" alt="A mesma questão com a prova original aberta: a página do PDF de verdade, com a questão 31 sob o título HISTÓRIA e a questão 33 ao lado"></td>
  </tr>
</table>

## O Jay-AI olha o seu resultado

Quando você termina, o Jay-S corrige a prova pelo gabarito, questão por questão (as de somatória valem pontos parciais), e manda o resultado para o Jay-AI, que escreve o que foi bem, o que revisar e o que fazer depois.

<p align="center"><img src="docs/images/result.png" alt="O resultado de uma tentativa na UEPG 2025 Grupo 1: 33,1 de 45, 74%, e a análise do Jay-AI embaixo, escrita em português, com um resumo e as seções do que foi bem e do que revisar" width="880"></p>

A análise é a leitura que o modelo faz das suas respostas, não uma nota: como todo modelo de linguagem, ele pode errar detalhes.

## Um chat que faz as contas

Pergunte ao Jay-AI sobre qualquer coisa, sobre os seus materiais ou sobre uma prova que você fez. Quando a pergunta é uma conta, o Jay-AI escolhe a fórmula e **o Jay-S faz a conta**: cada número é calculado pelo app, e num teste de hipótese a decisão (rejeitar H0 ou não) é a comparação que o app faz da estatística com o valor crítico. A resposta avisa com o selo "Calculado pelo app" e pede que você confira a fórmula no seu material.

<p align="center"><img src="docs/images/chat.png" alt="Uma conversa com o Jay-AI sobre uma apostila de estatística: a pergunta, em inglês, traz duas amostras (n = 40, média 72,5, desvio padrão 8; n = 50, média 68,9, desvio padrão 9) e pergunta se rejeita H0 ao nível de 5%; a resposta, com o selo Calculado pelo app, diz para rejeitar H0 porque |2,0062| é maior ou igual ao valor crítico 1,96, e mostra os passos, em inglês como a pergunta, com o erro padrão 1,7944 e Z = 2,0062" width="792"></p>

## Explicações e flashcards

**Explicar** transforma uma apostila num texto curto, pontos principais e cartões de conceitos, no idioma e na profundidade que você escolher, e depois faz perguntas para ver o que ficou. Os flashcards saem do seu material e se espaçam conforme você os avalia; uma avaliação pode ser desfeita, e cada sessão termina com um resumo.

<table>
  <tr>
    <td><img src="docs/images/explain.png" alt="A explicação do Jay-AI para uma apostila de uma página sobre o teste Z para duas médias, em inglês e no nível equilibrado: um título e cinco parágrafos sobre as hipóteses, a estatística, os valores críticos e o que significa não rejeitar H0"></td>
    <td><img src="docs/images/flashcards.png" alt="Um flashcard em revisão no baralho Comparing two means, faltando 8: o verso mostra a fórmula de Z para duas médias independentes, com os botões Esqueci (10 minutos), Difícil (12 horas), Bom (1 dia) e Fácil (4 dias), com a interface em inglês"></td>
  </tr>
</table>

## Três estilos, claro e escuro

Calmo, Neon e Suave, cada um com tema claro e escuro.

<table>
  <tr>
    <td><img src="docs/images/style-calm.png" alt="Os pontos principais e os cartões de conceitos da explicação do teste Z no estilo Calmo: fundo escuro com detalhes roxos e o lobo roxo"></td>
    <td><img src="docs/images/style-neon.png" alt="A mesma tela no estilo Neon: fundo quase preto com detalhes ciano e o lobo ciano"></td>
    <td><img src="docs/images/style-soft.png" alt="A mesma tela no estilo Suave: fundo escuro quente com detalhes laranja e o lobo laranja"></td>
  </tr>
  <tr align="center"><td>Calmo</td><td>Neon</td><td>Suave</td></tr>
</table>

<p align="center"><img src="docs/images/light.png" alt="A aba Provas prontas no tema claro com o estilo Suave: cartões brancos e botões laranja, Grupo 1 e Grupo 2 prontos para fazer" width="880"></p>


## Privado por natureza

Tudo roda no seu computador: o app, o Jay-AI e os seus dados. O Jay-S usa a rede só para três coisas:

- **O download do Jay-AI**, uma vez, quando você o liga: o programa e o modelo (1,3 ou 2,7 GB).
- **Uma prova pronta**, quando você clica no botão de baixar dela, do site da universidade.
- **A busca por atualização**, uma vez a cada abertura, nas releases deste repositório; a atualização só é instalada quando você clica nela.

O texto dos seus arquivos e do modelo aparece como texto, nunca como código da página, e a página nunca lida com caminhos de arquivo. A revisão de segurança está em [docs/SEGURANCA.md](docs/SEGURANCA.md).

Ele também é leve: com o Jay-AI parado, a versão de release mediu cerca de 110 a 120 MB de memória privada, e o Jay-AI sai da memória 2 minutos depois do último uso.

## Baixar

Baixe o `Jay-S_<versão>_x64-setup.exe` da **[última release](https://github.com/TyxMatty/Jay-S/releases/latest)** (Windows, 64 bits). O Jay-AI não vem no instalador: você o baixa no app, na seção Jay-AI. Todas as versões estão em [releases](https://github.com/TyxMatty/Jay-S/releases).

## Código

Tauri v2 com JavaScript puro na interface (`frontend/`) e um motor em Rust (`crates/jay-study`); o Jay-AI é o `llama-server` do llama.cpp, rodando num processo filho. O plano, as etapas e o que foi medido estão em [docs/ARQUITETURA.md](docs/ARQUITETURA.md).

```powershell
npm install
npm run dev    # o app (Tauri), Vite na porta 5178
npm run web    # só a interface no navegador
npm test
```

Licença: MIT.
