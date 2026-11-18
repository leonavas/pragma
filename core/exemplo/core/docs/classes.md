# Classes utilitárias

Gerado por `pragma-css` (`cargo run -p pragma-css --example classes_doc`); não edite à mão.

Subconjunto fechado do Tailwind v4: mesmos nomes, mesmo efeito. Classe fora desta lista é erro
no `pragma check`.

## Regras

- Proibido: valor arbitrário (`w-[37px]`), negativo (`-mt-2`) e qualquer classe fora da lista.
- Combinação que se repete em 3+ lugares, ou que precisa de algo fora da lista: crie uma classe
  custom. Global em `lib/theme.css` (`@layer components`); de componente em `<nome>.css`, com
  escopo `[data-c="nome"]`.
- Cores novas: declare `--color-<nome>` ou `--color-<nome>-<tom>` em `lib/theme.css`; elas
  entram em `C` (`bg-<nome>`, `text-<nome>-<tom>`…).
- Tokens redefiníveis em `lib/theme.css`: `--color-*`, `--font-{sans,serif,mono}`, `--radius-*`,
  `--shadow-*`, `--spacing`.
- Escuro: `[data-theme="dark"]` no `<html>`. A paleta não muda; troque o tom com `dark:`
  (`bg-white dark:bg-neutral-900`).

## Notação

`{a,b}` = alternativas; `{,-a}` = sem sufixo ou `-a`; `{1..12}` = inteiros.

- `S` espaçamento (n × 0.25rem; `px` = 1px): `0 px 0.5 1 1.5 2 2.5 3 3.5 4 5 6 7 8 9 10 11 12 14 16 20 24 28 32 36 40 44 48 52 56 60 64 72 80 96`
- `F` frações: `1/2 1/3 2/3 1/4 3/4`
- `C` cores: `white black transparent current {neutral,primary,danger,success,warning}-{50,100,200,300,400,500,600,700,800,900,950}`
- Opacidade em qualquer cor: `C/{10,20,30,40,50,60,70,80,90}` (ex.: `bg-primary-500/50`)

## Variantes

- Estado: `group-hover:` `hover:` `focus:` `focus-visible:` `active:` `disabled:` (`group-hover:` reage ao pai com `group`)
- Tema: `dark:`
- Responsivo, mobile first: `sm:` ≥40rem, `md:` ≥48rem, `lg:` ≥64rem, `xl:` ≥80rem, `2xl:` ≥96rem
- Combináveis, cada uma no máximo uma vez e um só breakpoint: `md:hover:bg-primary-600`

## Layout

- `block inline-block inline flex inline-flex grid inline-grid contents hidden`
- `static fixed absolute relative sticky`
- `{inset,inset-x,inset-y,top,right,bottom,left}-{S,F,auto,full}`
- `z-{0,10,20,30,40,50,auto}`
- `overflow{,-x,-y}-{auto,hidden,clip,visible,scroll}`
- `visible invisible`

## Flex e grid

- `flex-{row,row-reverse,col,col-reverse}`
- `flex-{wrap,nowrap,wrap-reverse}`
- `flex-{1,auto,initial,none}`
- `{grow,shrink}{,-0}`
- `basis-{S,F,auto,full}`
- `grid-cols-{1..12,none}`
- `grid-rows-{1..6,none}`
- `grid-flow-{row,col,dense}`
- `col-span-{1..12,full}`
- `row-span-{1..6,full}`
- `gap{,-x,-y}-S`
- `items-{start,end,center,baseline,stretch}`
- `justify-{start,end,center,between,around,evenly}`
- `justify-items-{start,end,center,stretch}`
- `self-{auto,start,end,center,stretch,baseline}`
- `place-items-{start,end,center,stretch}`

## Espaçamento

- `p{,x,y,t,r,b,l}-S`
- `m{,x,y,t,r,b,l}-{S,auto}`
- `space-{x,y}-S` — espaço entre filhos

## Tamanho

- `size-{S,F,auto,full,fit}`
- `w-{S,F,auto,full,screen,fit,min,max}`
- `min-w-{S,full,fit,min,max}`
- `max-w-{xs,sm,md,lg,xl,2xl,3xl,4xl,5xl,6xl,7xl,prose,full,screen,fit,none}`
- `h-{S,F,auto,full,screen,dvh,svh,fit,min,max}`
- `min-h-{S,full,screen,dvh,svh,fit}`
- `max-h-{S,full,screen,dvh,fit,none}`

## Tipografia

- `font-{sans,serif,mono}`
- `text-{xs,sm,base,lg,xl,2xl,3xl,4xl,5xl}` — tamanho com altura de linha
- `font-{thin,extralight,light,normal,medium,semibold,bold,extrabold,black}`
- `italic not-italic`
- `leading-{none,tight,snug,normal,relaxed,loose}`
- `tracking-{tighter,tight,normal,wide,wider,widest}`
- `text-{left,center,right,justify,start,end}`
- `uppercase lowercase capitalize normal-case`
- `underline line-through no-underline`
- `tabular-nums`
- `whitespace-{normal,nowrap,pre,pre-line,pre-wrap}`
- `break-{words,all}`
- `truncate`
- `line-clamp-{1..3}`
- `list-{none,disc,decimal}`

## Cor

- `{text,bg,border,ring,divide,fill,stroke}-C`

## Borda e efeitos

- `border{,-x,-y,-t,-r,-b,-l}{,-0,-2,-4,-8}`
- `border-{solid,dashed,dotted,none}`
- `divide-{x,y}{,-0,-2,-4,-8}` — borda entre filhos
- `rounded{,-t,-r,-b,-l}{,-none,-xs,-sm,-md,-lg,-xl,-2xl,-3xl,-full}`
- `ring{,-0,-1,-2,-4,-8}` — anel via box-shadow; cor com `ring-C`
- `shadow{,-2xs,-xs,-sm,-md,-lg,-xl,-2xl,-none}`
- `opacity-{0,5,10,20,25,30,40,50,60,70,75,80,90,95,100}`
- `outline-{none,hidden}`

## Interação e acessibilidade

- `cursor-{auto,default,pointer,wait,text,move,not-allowed,grab,grabbing}`
- `pointer-events-{none,auto}`
- `select-{none,text,all,auto}`
- `sr-only not-sr-only`

## Transição e mídia

- `transition{,-all,-colors,-opacity,-shadow,-transform,-none}`
- `duration-{150,200,300}`
- `aspect-{auto,square,video}`
- `object-{contain,cover,fill,none,scale-down}`
- `object-{center,top,bottom,left,right}`

## Outras classes reconhecidas

- `group` — marca o pai para `group-hover:`
- `skeleton` — bloco pulsante de carregamento (vem do `base.css`)
