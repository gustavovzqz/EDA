# EDA

Realizado por Gustavo Fernandez Vidal Vazquez, MDCC, Matrícula 603150.

## Especificações

Realizado em Rust, mais especificamente com a versão do `rustc` abaixo:

```
rustc 1.95.0 (59807616e 2026-04-14) (Arch Linux rust 1:1.95.0-1)
```

## Instruções de Compilação / Execução

### Para Compilar

```bash
make build
```

### Executar passando uma entrada:

```bash
make run INPUT="entrada"
```

### Limpar o binário gerado:

```bash
make clean
```


# Árvore Rubro Negra com Persistência Parcial

Todas as as funções e estruturas referenciadas abaixo estão em `src/main.rs`.

## Nós

O código em Rust, implementa a persistência parcial usando uma estrura de nós, que contam com:
1. Filho esquerdo, direito;
2. Valor;
3. Cor;
4. Ponteiro para o Pai;
5. Vetor de modificações.

```rust
struct Node {
    value: i32,
    left: Link,
    right: Link,
    color: Color,
    parent: RefCell<Option<(Weak<Node>, Side)>>,
    mods: RefCell<Vec<Mod>>,
}
```

A ideia é que eu tenha um `Rc<>` para os filhos (similar ao `shared_pointer` em C++) e um ponteiro `Weak` para o pai, evitando ciclo de referências. Apenas o ponteiro para o pai e o vetor de mods são mutáveis. O ponteiro para o pai também possui um side, que diz se sou filho esquerdo ou direito.


Um mod é um vetor de modificações, modificações podem ser de três tipos:

``` rust
enum ModKind {
    Position(Side, Link),
    Color(Color),
    Value(i32),
}
```

Posição é responsável por mudar um filho, recebe a posição onde ele vai ser inserido e o ponteiro para o nó. A cor e o valor são propriedades básicas do nó.

### Alterando / Consultando um Nó

#### Update
```rust
  fn update_with_node(
        self: &Rc<Self>,
        kind: ModKind, # Mudar Filho, valor ou Cor do nó
        version: u32,
    ) -> (Rc<Node>, Option<Rc<Node>>) {
```

Para alterar um nó, usamos o método `update(versão, mudança)`, que recebe a última versão da estrutura e a mudança desejada. Temos dois casos:

1. Se há espaço em MODS, apenas inserimos a mudança lá. Após cada mudança, os filhos que aponto ou que deixei de apontar têm os `back_pointers` atualizados.
2. Se não há espaço em MODS, crio um nó novo e ajusto o ponteiro do pai para apontar para esse nó novo.

Ao final do update, eu retorno uma versão fresca do nó que acabei de alterar, e um Option: `None` se a raíz não mudou e `Some(raiz)` caso a raíz tenha alterado.

Opcionalmente, tenho uma versão do update que não retorna o nó alterado, apenas a raiz. 

#### Consultando

```rust
fn get_color(&self, version: u32) -> Color 
```

Para obter qualquer informação do nó, podemos usar o `get` associado (`get_color, get_value...`). O `get` percorre o vetor de MODS e o próprio nó para obter o presente na versão `version`. 

## Operações na Árvore

Com a base acima, já temos o suficiente para trabalhar com árvores persistentes, o ponto é usar os métodos de forma cuidadosa e não referenciar algum nó de outra versão.

### Operações de Busca (Sucessor / Imprimir)

Como as operações de busca não modificam a árvore, basta tomar cuidado para usar os métodos `get` para obter os dados da versão específica do nó, Dessa forma, não há nada especial em operações de busca, se você parte da raiz correta e usa os métodos de acesso, você está vendo os nós da versão especificada.
### Operações que Modificam (Incluir / Deletar)

**Todas as operações em Árvore Rubro Negra foram adaptadas do livro Introduction to Algorithms, Quarta Edição : Thomas H. Cormen, Charles E. Leiserson, Ronald L. Rivest e
Clifford Stein.**

---

A maior preocupação ao fazer uma operação na Árvore é manter a raiz atualizada. Como cada operação pode ou não gerar uma raiz nova, as operações que podem modificar a raiz sempre retornam `None`, caso a raiz da nova versão seja o mesmo nó, ou `Some(r)`, caso o nó `r` agora é o representante da nova versão. Isso é útil para a estrutura que mantém as raízes.

Ao fazer operações em cadeia, como o `Left_Rotate`, preciso tomar muito cuidado para não referenciar um nó possivelmente antigo. 

Se eu faço `x.update(algo)` e logo em sequência desejo alterar o `x` novamente, preciso usar a versão mais recente do x, que é retornada no update. Assim, meu código está repleto de trechos assim:

```rust
 let (ny, r_y) = y.update_with_node(ModKind::Position(Side::Left, b), version);
```

Isso basicamente diz, `ny` é o novo y, após a inserção, (pode ser o mesmo, caso não tenha sido copiado), e `r_y` é `None`, se a raiz não mudou, e `Some(r)`, se a raiz nova é r. Assim, todas mudanças em "cadeia" (como rotate), precisam acumular as raízes para conseguir retornar a raiz mais nova. 

Os trechos `current_root = current_root.or(new_root)` servem para que `current_root` sempre referencie a última raiz.

Todas as operações de árvore seguem a mesma ideia, mas com complexidades diferentes. Não cabe aqui explorar a lógica de cada uma, mas a ideia geral é:

1. Se eu atualizei um nó X, possivelmente minha referência a ele a todos os ancestrais deles estão desatualizadas.
2. Para não percorrer a árvore inteira a partir da raiz nova, eu uso "âncoras" ou nós confiáveis para acessar a informação que eu preciso.

Assim, a ideia de incluir/remover é essencialmente a mesma de uma árvore não persistente, mas buscando sempre as referências corretas aos nós da versão mais recente e a manutenção das raízes.

## Estrutura de Manutenção das Raízes

```rust
struct PersistentStructure {
    roots: HashMap<u32, Rc<Node>>,

    current_version: u32,
}
```

A estrutura de manutenção das raízes é bem simples. A ideia é realizar o seguinte fluxo:

Acesso a raiz da versão atual usando o `HashMap`. A escolha do `HashMap` aqui é arbitrária, poderia ser um vetor fixo (já que teremos no máximo 100 versões) ou um vetor dinâmico. Possivelmente alguma estrutura que lida com "ranges" seria o melhor, já que temos um algo do tipo "da versão 0 até a 4 o representante é a raiz (...)"

De qualquer forma, após acessar a raiz temos:

1. Se for uma operação de busca, buscamos usando a raiz e nada é alterado.
2. Se estamos alterando algo na árvore, verificamos se uma raiz nova foi retornada. Se a operação criou uma raiz nova, adicionamos uma entrada no `HashMap` associando a versão nova com a raiz nova. Se uma raiz não foi criada, adicionamos uma entrada no `HashMap` associando a versão nova com a raiz que se manteve.






