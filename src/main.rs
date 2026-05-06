use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

type Link = Option<Rc<Node>>;
const MAX_MODS_SIZE: usize = 0; // Path Copying Puro

#[derive(Clone, Copy, Debug, PartialEq)]
enum Side {
    Left,
    Right,
}

#[derive(Clone, Copy, PartialEq)]
enum Color {
    Red,
    Black,
}

#[derive(Clone)]
enum ModKind {
    Position(Side, Link),
    Color(Color),
    Value(i32),
}

#[derive(Clone)]
struct Mod {
    version: u32,
    kind: ModKind,
}

struct Node {
    value: i32,
    left: Link,
    right: Link,
    color: Color,
    parent: RefCell<Option<(Weak<Node>, Side)>>,
    mods: RefCell<Vec<Mod>>,
}

impl Node {
    fn get_left(&self, version: u32) -> Link {
        let mods = self.mods.borrow();
        for m in mods.iter().rev() {
            if m.version <= version {
                if let ModKind::Position(Side::Left, ref l) = m.kind {
                    return l.clone();
                }
            }
        }
        self.left.clone()
    }

    fn get_right(&self, version: u32) -> Link {
        let mods = self.mods.borrow();
        for m in mods.iter().rev() {
            if m.version <= version {
                if let ModKind::Position(Side::Right, ref r) = m.kind {
                    return r.clone();
                }
            }
        }
        self.right.clone()
    }

    fn get_value(&self, version: u32) -> i32 {
        let mods = self.mods.borrow();
        for m in mods.iter().rev() {
            if m.version <= version {
                if let ModKind::Value(v) = m.kind {
                    return v;
                }
            }
        }
        self.value
    }

    fn get_color(&self, version: u32) -> Color {
        let mods = self.mods.borrow();
        for m in mods.iter().rev() {
            if m.version <= version {
                if let ModKind::Color(v) = m.kind {
                    return v;
                }
            }
        }
        self.color
    }

    // Nó que alterei + raiz
    fn update_with_node(
        self: &Rc<Self>,
        kind: ModKind,
        version: u32,
    ) -> (Rc<Node>, Option<Rc<Node>>) {
        let mut mods = self.mods.borrow_mut();

        if mods.len() < MAX_MODS_SIZE {
            mods.push(Mod { version, kind });
            return (self.clone(), None);
        }

        drop(mods);

        let mut value = self.get_value(version);
        let mut left = self.get_left(version);
        let mut right = self.get_right(version);
        let mut color = self.get_color(version);

        match kind {
            ModKind::Value(v) => value = v,
            ModKind::Position(Side::Left, l) => left = l,
            ModKind::Position(Side::Right, r) => right = r,
            ModKind::Color(c) => color = c,
        }

        let parent_info = self.parent.borrow().clone();

        let new_node = Rc::new(Node {
            value,
            color,
            left: left.clone(),
            right: right.clone(),
            parent: RefCell::new(parent_info.clone()),
            mods: RefCell::new(vec![]),
        });

        // ATUALIZAÇÃO DOS FILHOS: Crucial para o Path Copying
        if let Some(ref l) = left {
            *l.parent.borrow_mut() = Some((Rc::downgrade(&new_node), Side::Left));
        }
        if let Some(ref r) = right {
            *r.parent.borrow_mut() = Some((Rc::downgrade(&new_node), Side::Right));
        }

        if let Some((parent_weak, side)) = parent_info {
            if let Some(parent_rc) = parent_weak.upgrade() {
                let mod_to_propagate = match side {
                    Side::Left => ModKind::Position(Side::Left, Some(new_node.clone())),
                    Side::Right => ModKind::Position(Side::Right, Some(new_node.clone())),
                };
                let (_, final_root) = parent_rc.update_with_node(mod_to_propagate, version);
                return (new_node, final_root);
            }
        }
        (new_node.clone(), Some(new_node))
    }

    fn update(self: &Rc<Self>, kind: ModKind, version: u32) -> Option<Rc<Node>> {
        let (_, root) = self.update_with_node(kind, version);
        root
    }
}

fn left_rotate(x: &Rc<Node>, version: u32) -> (Option<Rc<Node>>, Rc<Node>) {
    let y = x.get_right(version).expect("Rotação exige filho direito");
    let a = x.get_left(version);
    let b = y.get_left(version);
    let g = y.get_right(version);

    let x_val = x.get_value(version);
    let x_col = x.get_color(version);
    let y_val = y.get_value(version);
    let y_col = y.get_color(version);

    let mut root_acc = None;

    // 1. NY é quem assume o valor/cor de X e os filhos A e B.
    // ESTE é o nó que você quer retornar (quem "se tornou o x")
    let (ny, r) = y.update_with_node(ModKind::Value(x_val), version);
    root_acc = r.or(root_acc);
    let (ny, r) = ny.update_with_node(ModKind::Color(x_col), version);
    root_acc = r.or(root_acc);
    let (ny, r) = ny.update_with_node(ModKind::Position(Side::Left, a), version);
    root_acc = r.or(root_acc);
    let (ny, r) = ny.update_with_node(ModKind::Position(Side::Right, b), version);
    root_acc = r.or(root_acc);

    // 2. NX (nó físico x) assume a identidade de Y
    let (nx, r) = x.update_with_node(ModKind::Value(y_val), version);
    root_acc = r.or(root_acc);
    let (nx, r) = nx.update_with_node(ModKind::Color(y_col), version);
    root_acc = r.or(root_acc);
    let (nx, r) = nx.update_with_node(ModKind::Position(Side::Right, g), version);
    root_acc = r.or(root_acc);

    // 3. Link final
    let (_, r) = nx.update_with_node(ModKind::Position(Side::Left, Some(ny.clone())), version);
    root_acc = r.or(root_acc);

    // Retorna a raiz acumulada e o NY (o nó que desceu com a identidade de x)
    (root_acc, ny)
}
fn right_rotate(y: &Rc<Node>, version: u32) -> (Option<Rc<Node>>, Rc<Node>) {
    let x = y.get_left(version).expect("Rotação exige filho esquerdo");
    let a = x.get_left(version);
    let b = x.get_right(version);
    let g = y.get_right(version);

    let y_val = y.get_value(version);
    let y_col = y.get_color(version);
    let x_val = x.get_value(version);
    let x_col = x.get_color(version);

    let mut root_acc = None;

    // 1. X assume identidade de Y
    let (nx, r) = x.update_with_node(ModKind::Value(y_val), version);
    root_acc = r.or(root_acc);
    let (nx, r) = nx.update_with_node(ModKind::Color(y_col), version);
    root_acc = r.or(root_acc);
    let (nx, r) = nx.update_with_node(ModKind::Position(Side::Right, g), version);
    root_acc = r.or(root_acc);
    let (nx, r) = nx.update_with_node(ModKind::Position(Side::Left, b), version);
    root_acc = r.or(root_acc);

    // 2. Y assume identidade de X
    let (ny, r) = y.update_with_node(ModKind::Value(x_val), version);
    root_acc = r.or(root_acc);
    let (ny, r) = ny.update_with_node(ModKind::Color(x_col), version);
    root_acc = r.or(root_acc);
    let (ny, r) = ny.update_with_node(ModKind::Position(Side::Left, a), version);
    root_acc = r.or(root_acc);

    // 3. Link final
    let (_, r) = ny.update_with_node(ModKind::Position(Side::Right, Some(nx.clone())), version);
    root_acc = r.or(root_acc);

    (root_acc, nx)
} // --- FUNÇÕES AUXILIARES ---

fn find_parent_for_insertion(
    root: &Link,
    value: i32,
    version: u32,
    last_parent: Option<Rc<Node>>,
) -> Option<Rc<Node>> {
    match root {
        Some(node) => {
            let v = node.get_value(version);
            if value <= v {
                find_parent_for_insertion(
                    &node.get_left(version),
                    value,
                    version,
                    Some(node.clone()),
                )
            } else {
                find_parent_for_insertion(
                    &node.get_right(version),
                    value,
                    version,
                    Some(node.clone()),
                )
            }
        }
        None => last_parent,
    }
}

fn find_node(root: &Link, value: i32, version: u32) -> Option<Rc<Node>> {
    match root {
        Some(node) => {
            let v = node.get_value(version);
            if value < v {
                find_node(&node.get_left(version), value, version)
            } else if value > v {
                find_node(&node.get_right(version), value, version)
            } else {
                Some(node.clone())
            }
        }
        None => None,
    }
}

fn find_min(root: &Rc<Node>, version: u32) -> Rc<Node> {
    if let Some(left) = root.get_left(version) {
        find_min(&left, version)
    } else {
        Rc::clone(root)
    }
}

fn rb_insert_fixup(z: &Rc<Node>, version: u32, current_root: &Rc<Node>) -> Rc<Node> {
    let mut current = z.clone();
    let mut root_acc: Option<Rc<Node>> = None;

    let get_parent_info = |node: &Rc<Node>| node.parent.borrow().clone();

    while let Some((p_weak, _)) = get_parent_info(&current) {
        let p = match p_weak.upgrade() {
            Some(node) => node,
            None => break,
        };

        if p.get_color(version) == Color::Black {
            break;
        }

        let (gp_weak, side_p_to_gp) =
            get_parent_info(&p).expect("RBT Erro: Pai vermelho exige avô");
        let gp = gp_weak.upgrade().expect("Avô deve estar vivo");

        if side_p_to_gp == Side::Left {
            let y = gp.get_right(version);

            if y.as_ref().map_or(Color::Black, |n| n.get_color(version)) == Color::Red {
                // --- CASO 1: RECOLORAÇÃO ---
                // Tio primeiro
                if let Some(uncle) = y {
                    let (_, r2) = uncle.update_with_node(ModKind::Color(Color::Black), version);
                    root_acc = r2.or(root_acc);
                }
                // Pai por último (Bússola)
                let (new_p, r1) = p.update_with_node(ModKind::Color(Color::Black), version);
                root_acc = r1.or(root_acc);

                let (gp_fresh_w, _) = get_parent_info(&new_p).expect("Avô deve existir");
                let gp_fresh = gp_fresh_w.upgrade().expect("Avô deve estar vivo");

                let (new_gp, r3) = gp_fresh.update_with_node(ModKind::Color(Color::Red), version);
                root_acc = r3.or(root_acc);

                current = new_gp;
            } else {
                // --- CASO 2: ROTAÇÃO SIMPLES ---
                if matches!(get_parent_info(&current), Some((_, Side::Right))) {
                    current = p.clone();
                    let (new_r, updated_node) = left_rotate(&current, version);
                    root_acc = new_r.or(root_acc);
                    current = updated_node;
                }

                // --- CASO 3: ROTAÇÃO FINAL ---
                let (p_f_w, _) = get_parent_info(&current).expect("Pai sumiu");
                let p_f = p_f_w.upgrade().unwrap();

                let (new_p, r_p) = p_f.update_with_node(ModKind::Color(Color::Black), version);
                root_acc = r_p.or(root_acc);

                let (gp_f_w, _) = get_parent_info(&new_p).expect("Avô sumiu");
                let gp_f = gp_f_w.upgrade().unwrap();

                let (gp_red, r_gp) = gp_f.update_with_node(ModKind::Color(Color::Red), version);
                root_acc = r_gp.or(root_acc);

                let (new_r, _) = right_rotate(&gp_red, version);
                root_acc = new_r.or(root_acc);
                break;
            }
        } else {
            // --- LÓGICA ESPELHADA (P é Side::Right) ---
            let y = gp.get_left(version);

            if y.as_ref().map_or(Color::Black, |n| n.get_color(version)) == Color::Red {
                // --- CASO 1 ESPELHADO: RECOLORAÇÃO ---
                // 1. Tio primeiro
                if let Some(uncle) = y {
                    let (_, r2) = uncle.update_with_node(ModKind::Color(Color::Black), version);
                    root_acc = r2.or(root_acc);
                }
                // 2. Pai por último (Bússola)
                let (new_p, r1) = p.update_with_node(ModKind::Color(Color::Black), version);
                root_acc = r1.or(root_acc);

                // 3. Re-ancoragem do Avô via Pai novo
                let (gp_f_w, _) = get_parent_info(&new_p).expect("Avô deve existir");
                let gp_f = gp_f_w.upgrade().expect("Avô deve estar vivo");

                // 4. Avô fica Red e subimos
                let (new_gp, r3) = gp_f.update_with_node(ModKind::Color(Color::Red), version);
                root_acc = r3.or(root_acc);
                current = new_gp;
            } else {
                // --- CASO 2 ESPELHADO ---
                if matches!(get_parent_info(&current), Some((_, Side::Left))) {
                    current = p.clone();
                    let (new_r, updated_node) = right_rotate(&current, version);
                    root_acc = new_r.or(root_acc);
                    current = updated_node;
                }
                // --- CASO 3 ESPELHADO ---
                let (p_f_w, _) = get_parent_info(&current).expect("Pai sumiu");
                let p_f = p_f_w.upgrade().expect("Pai deve estar vivo");

                let (new_p, r_p) = p_f.update_with_node(ModKind::Color(Color::Black), version);
                root_acc = r_p.or(root_acc);

                let (gp_f_w, _) = get_parent_info(&new_p).expect("Avô sumiu");
                let gp_f = gp_f_w.upgrade().expect("Avô deve estar vivo");

                let (gp_red, r_gp) = gp_f.update_with_node(ModKind::Color(Color::Red), version);
                root_acc = r_gp.or(root_acc);

                let (new_r, _) = left_rotate(&gp_red, version);
                root_acc = new_r.or(root_acc);
                break;
            }
        }
    }

    let latest_root = root_acc.unwrap_or_else(|| current_root.clone());
    latest_root
        .update(ModKind::Color(Color::Black), version)
        .unwrap_or(latest_root)
}

fn insert(root: &Rc<Node>, value: i32, version: u32) -> Option<Rc<Node>> {
    // 1. Localiza o pai para a inserção
    if let Some(parent) = find_parent_for_insertion(&Some(root.clone()), value, version, None) {
        let parent_value = parent.get_value(version);

        // 2. Cria o novo nó (sempre Vermelho)
        let new_node = Rc::new(Node {
            value,
            color: Color::Red,
            left: None,
            right: None,
            parent: RefCell::new(None),
            mods: RefCell::new(vec![]),
        });

        let side = if value <= parent_value {
            Side::Left
        } else {
            Side::Right
        };

        // Define o pai do novo nó
        *new_node.parent.borrow_mut() = Some((Rc::downgrade(&parent), side));

        // 3. Conecta o novo nó ao pai e captura a possível nova raiz intermediária
        // Se update retornar None, a raiz continua sendo a 'root' original
        let root_after_insertion = parent
            .update(ModKind::Position(side, Some(new_node.clone())), version)
            .unwrap_or_else(|| root.clone());

        // 4. Executa o balanceamento (Fixup)
        // O fixup agora recebe o nó novo, a versão e a raiz atualizada
        let final_root = rb_insert_fixup(&new_node, version, &root_after_insertion);

        Some(final_root)
    } else {
        // Se não encontrou pai, a árvore estava vazia (tratado no PersistentStructure)
        None
    }
}

fn rb_delete_fixup(x_init: Option<Rc<Node>>, version: u32) -> Option<Rc<Node>> {
    let mut current_x = x_init;
    let mut root_acc: Option<Rc<Node>> = None;

    let get_parent_info = |node: &Rc<Node>| node.parent.borrow().clone();

    // O loop continua enquanto x não for raiz e x for PRETO
    while let Some(x_node) = current_x.clone() {
        // Condição de parada: se x é RED ou se x é ROOT (não tem pai)
        if x_node.get_color(version) == Color::Red || x_node.parent.borrow().is_none() {
            break;
        }

        // 1. Pega o PAI atualizado direto do x.
        // Se algo rotacionou, o back-pointer do x já aponta para o clone novo.
        let (p_weak, side) = get_parent_info(&x_node).expect("x deve ter pai");
        let cp = p_weak.upgrade().expect("Memória do pai deve estar ativa");

        if side == Side::Left {
            let mut w = cp.get_right(version).expect("Irmão deve existir");

            // --- CASO 1: Irmão é RED ---
            if w.get_color(version) == Color::Red {
                let (new_w, r1) = w.update_with_node(ModKind::Color(Color::Black), version);
                root_acc = r1.or(root_acc);

                // Re-ancora o pai através do novo w para manter a linhagem
                let (p_f, _) = get_parent_info(&new_w).unwrap();
                let (new_p, r2) = p_f
                    .upgrade()
                    .unwrap()
                    .update_with_node(ModKind::Color(Color::Red), version);
                root_acc = r2.or(root_acc);

                let (new_root, _) = left_rotate(&new_p, version);
                root_acc = new_root.or(root_acc);

                // Após rotação, o "w" mudou. Pegamos o novo irmão do pai atualizado.
                // O pai de x mudou, mas x_node ainda aponta para o mesmo nó cujo back-pointer se ajustou.
                let (p_f2, _) = get_parent_info(&x_node).unwrap();
                w = p_f2
                    .upgrade()
                    .unwrap()
                    .get_right(version)
                    .expect("Novo irmão");
            }

            let w_l_c = w
                .get_left(version)
                .map_or(Color::Black, |n| n.get_color(version));
            let w_r_c = w
                .get_right(version)
                .map_or(Color::Black, |n| n.get_color(version));

            // --- CASO 2: Sobrinhos são BLACK ---
            if w_l_c == Color::Black && w_r_c == Color::Black {
                let (new_w, r) = w.update_with_node(ModKind::Color(Color::Red), version);
                root_acc = r.or(root_acc);

                // x SOBE: o novo x é o pai atualizado
                let (p_f, _) = get_parent_info(&new_w).unwrap();
                current_x = Some(p_f.upgrade().unwrap());
                // O loop reavaliará o novo current_x (o pai)
            } else {
                // --- CASO 3: Sobrinho oposto (direito) é BLACK ---
                if w_r_c == Color::Black {
                    if let Some(wl) = w.get_left(version) {
                        let (new_wl, r) =
                            wl.update_with_node(ModKind::Color(Color::Black), version);
                        root_acc = r.or(root_acc);
                        let (w_f, _) = get_parent_info(&new_wl).unwrap();
                        w = w_f.upgrade().unwrap();
                    }
                    let (new_w, r) = w.update_with_node(ModKind::Color(Color::Red), version);
                    root_acc = r.or(root_acc);

                    let (new_root, updated_w) = right_rotate(&new_w, version);
                    root_acc = new_root.or(root_acc);

                    let (p_f, _) = get_parent_info(&updated_w).expect("w rotacionado deve ter pai");
                    w = p_f.upgrade().expect("Pai do w rotacionado sumiu");
                }

                // --- CASO 4: Rotação Final ---
                // Pegamos o pai atualizado através do x_node uma última vez
                let (p_f_final, _) = get_parent_info(&x_node).unwrap();
                let p_actual = p_f_final.upgrade().unwrap();

                let p_color = p_actual.get_color(version);

                // 1. Pinta o irmão (w) com a cor do pai
                let (new_w, r1) = w.update_with_node(ModKind::Color(p_color), version);
                root_acc = r1.or(root_acc);

                // 2. Pinta o sobrinho (wr) de preto
                let mut final_w = new_w;
                if let Some(wr) = final_w.get_right(version) {
                    let (new_wr, r3) = wr.update_with_node(ModKind::Color(Color::Black), version);
                    root_acc = r3.or(root_acc);

                    // IMPORTANTE: O sobrinho mudou, então pegamos o novo "pai" dele (o irmão w)
                    let (w_f, _) = get_parent_info(&new_wr).unwrap();
                    final_w = w_f.upgrade().unwrap();
                }

                // 3. Pinta o pai de preto
                // AGORA A CHAVE: Usamos o back-pointer do 'final_w' para achar o pai que
                // já está conectado ao irmão e sobrinho novos.
                let (p_f_final, _) = get_parent_info(&final_w).expect("Irmão deve ter pai");
                let (new_p_para_rotacao, r2) = p_f_final
                    .upgrade()
                    .unwrap()
                    .update_with_node(ModKind::Color(Color::Black), version);
                root_acc = r2.or(root_acc);

                // 4. Rotação final
                // Usamos o 'new_p_para_rotacao' porque ele é o único que "viu"
                // a cor preta chegar no sobrinho através do final_w.
                let (new_root, _) = left_rotate(&new_p_para_rotacao, version);
                root_acc = new_root.or(root_acc);

                current_x = root_acc.clone();
                break;
            }
        } else {
            // --- LÓGICA ESPELHADA (Side::Right) ---
            let mut w = cp.get_left(version).expect("Irmão deve existir");

            // --- CASO 1: Irmão é RED ---
            if w.get_color(version) == Color::Red {
                let (new_w, r1) = w.update_with_node(ModKind::Color(Color::Black), version);
                root_acc = r1.or(root_acc);

                let (p_f, _) = get_parent_info(&new_w).unwrap();
                let (new_p, r2) = p_f
                    .upgrade()
                    .unwrap()
                    .update_with_node(ModKind::Color(Color::Red), version);
                root_acc = r2.or(root_acc);

                let (new_root, _) = right_rotate(&new_p, version);
                root_acc = new_root.or(root_acc);

                // Re-sincroniza w: pega o novo irmão esquerdo do pai de x
                let (p_f2, _) = get_parent_info(&x_node).unwrap();
                w = p_f2
                    .upgrade()
                    .unwrap()
                    .get_left(version)
                    .expect("Novo irmão");
            }

            let w_l_c = w
                .get_left(version)
                .map_or(Color::Black, |n| n.get_color(version));
            let w_r_c = w
                .get_right(version)
                .map_or(Color::Black, |n| n.get_color(version));

            // --- CASO 2: Sobrinhos são BLACK ---
            if w_l_c == Color::Black && w_r_c == Color::Black {
                let (new_w, r) = w.update_with_node(ModKind::Color(Color::Red), version);
                root_acc = r.or(root_acc);

                let (p_f, _) = get_parent_info(&new_w).unwrap();
                current_x = Some(p_f.upgrade().unwrap());
            } else {
                // --- CASO 3: Sobrinho oposto (esquerdo) é BLACK ---
                // No Side::Right, o sobrinho oposto é o LEFT.
                if w_l_c == Color::Black {
                    if let Some(wr) = w.get_right(version) {
                        let (new_wr, r) =
                            wr.update_with_node(ModKind::Color(Color::Black), version);
                        root_acc = r.or(root_acc);
                        let (w_f, _) = get_parent_info(&new_wr).unwrap();
                        w = w_f.upgrade().unwrap();
                    }
                    let (new_w, r) = w.update_with_node(ModKind::Color(Color::Red), version);
                    root_acc = r.or(root_acc);

                    let (new_root, node_below) = left_rotate(&new_w, version);
                    root_acc = new_root.or(root_acc);

                    // O novo irmão w é o pai do nó que desceu na rotação
                    let (p_f, _) = get_parent_info(&node_below).expect("w deve ter pai");
                    w = p_f.upgrade().expect("Pai do w sumiu");
                }

                // --- CASO 4: Rotação Final ---
                let (p_f_final, _) = get_parent_info(&x_node).unwrap();
                let p_actual = p_f_final.upgrade().unwrap();
                let p_color = p_actual.get_color(version);

                // 1. Irmão recebe cor do pai
                let (new_w, r1) = w.update_with_node(ModKind::Color(p_color), version);
                root_acc = r1.or(root_acc);

                // 2. Sobrinho (wl) fica preto e re-ancora w
                let mut final_w = new_w;
                if let Some(wl) = final_w.get_left(version) {
                    let (new_wl, r3) = wl.update_with_node(ModKind::Color(Color::Black), version);
                    root_acc = r3.or(root_acc);
                    let (w_f, _) = get_parent_info(&new_wl).unwrap();
                    final_w = w_f.upgrade().unwrap();
                }

                // 3. Pai fica preto e re-ancora via final_w
                let (p_f_last, _) = get_parent_info(&final_w).expect("w deve ter pai");
                let (new_p_final, r2) = p_f_last
                    .upgrade()
                    .unwrap()
                    .update_with_node(ModKind::Color(Color::Black), version);
                root_acc = r2.or(root_acc);

                // 4. Rotação final (Right Rotate no pai)
                let (new_root, _) = right_rotate(&new_p_final, version);
                root_acc = new_root.or(root_acc);

                current_x = root_acc.clone();
                break;
            }
        }
    }

    // Linha final do algoritmo: pinta x de preto
    if let Some(x) = current_x {
        let final_r = x.update(ModKind::Color(Color::Black), version);
        return final_r.or(root_acc);
    }
    root_acc
}
fn remove(node_to_remove: &Rc<Node>, version: u32) -> Option<Rc<Node>> {
    let color_removed = node_to_remove.get_color(version); // 1. Guardamos a cor original
    let left_child = node_to_remove.get_left(version);
    let right_child = node_to_remove.get_right(version);
    let parent_info = node_to_remove.parent.borrow().clone();

    // --- CASO DE 2 FILHOS ---
    if left_child.is_some() && right_child.is_some() {
        let succ = find_min(right_child.as_ref().unwrap(), version);
        let val = succ.get_value(version);

        // Path copying: atualiza o valor e gera nova raiz (root_after_val)
        let root_after_val = node_to_remove.update(ModKind::Value(val), version);

        // Remove o sucessor na árvore recém-clonada
        let final_root = remove(&succ, version);
        return final_root.or(root_after_val);
    }

    // --- CASO DE 0 OU 1 FILHO ---
    // 'x' é o nó que vai ocupar o lugar do deletado
    let x = left_child.or(right_child);

    // Caso de Raiz Física -> x é a nova raiz
    if parent_info.is_none() {
        return if let Some(new_root) = x {
            *new_root.parent.borrow_mut() = None;
            // A nova raiz deve ser sempre Preta
            let (new_, _) = new_root.update_with_node(ModKind::Color(Color::Black), version);
            Some(new_)
        } else {
            None
        };
    }

    let (parent_weak, side) = parent_info.unwrap();
    let parent_rc = parent_weak.upgrade().expect("Pai deve existir");

    // 2. Realizamos a troca física usando update_with_node para obter o pai novo (clone)
    let (_, root_after_pos) =
        parent_rc.update_with_node(ModKind::Position(side, x.clone()), version);

    // 3. SE O NÓ REMOVIDO ERA PRETO -> FIXUP
    if color_removed == Color::Black {
        // O fixup retorna a nova raiz se o balanceamento subir até o topo
        return rb_delete_fixup(x, version).or(root_after_pos);
    }

    root_after_pos
}

struct PersistentStructure {
    roots: HashMap<u32, Rc<Node>>,
    current_version: u32,
}

impl PersistentStructure {
    fn new() -> Self {
        Self {
            roots: HashMap::new(),
            current_version: 0,
        }
    }

    fn insert(&mut self, value: i32) {
        let old_v = self.current_version;
        let new_v = old_v + 1;

        if let Some(root) = self.roots.get(&old_v).cloned() {
            let res = insert(&root, value, new_v);
            self.roots.insert(new_v, res.unwrap_or(root));
        } else {
            let root = Rc::new(Node {
                value,
                color: Color::Black,
                left: None,
                right: None,
                parent: RefCell::new(None),
                mods: RefCell::new(vec![]),
            });
            self.roots.insert(new_v, root);
        }
        self.current_version = new_v;
    }

    fn remove(&mut self, value: i32) {
        let old_v = self.current_version;
        let new_v = old_v + 1;

        if let Some(root) = self.roots.get(&old_v).cloned() {
            if let Some(node) = find_node(&Some(root.clone()), value, old_v) {
                if let Some(new_root) = remove(&node, new_v) {
                    self.roots.insert(new_v, new_root);
                }
            } else {
                self.roots.insert(new_v, root);
            }
        }

        self.current_version = new_v;
    }

    fn print(&self, version: u32) {
        println!("--- Visualizando Versão {} ---", version);
        if let Some(root) = self.roots.get(&version) {
            Self::print_rec(&Some(root.clone()), version, 0);
        } else {
            println!("[Árvore Vazia]");
        }
    }

    fn print_rec(link: &Link, v: u32, depth: usize) {
        if let Some(n) = link {
            // Imprime a subárvore direita primeiro (topo do console)
            Self::print_rec(&n.get_right(v), v, depth + 1);

            // Lógica de cor
            let color_code = match n.get_color(v) {
                Color::Red => "R",
                Color::Black => "B",
            };

            // Imprime o valor com a cor ao lado: ex "40(B)"
            println!("{}{}({})", "    ".repeat(depth), n.get_value(v), color_code);

            // Imprime a subárvore esquerda
            Self::print_rec(&n.get_left(v), v, depth + 1);
        }
    }
}

use std::io::{self, Write};

// Assumindo que você tem um Enum Color
// #[derive(Debug, Clone, Copy, PartialEq)]
// enum Color { Red, Black }

fn main() {
    let mut ps = PersistentStructure::new();

    println!("=== RBT PERSISTENTE: CLI v2.0 ===");
    println!("Comandos:");
    println!("  <numero>    -> Insere o valor");
    println!("  r <numero>  -> Remove o valor");
    println!("  v <numero>  -> Visualiza uma versão específica");
    println!("  sair        -> Encerra o programa");

    loop {
        println!("\n========================================");
        println!(
            "VERSÃO ATUAL: {:2} | Nó Raiz: {:p}",
            ps.current_version,
            ps.roots
                .get(&ps.current_version)
                .map_or(std::ptr::null(), |r| Rc::as_ptr(r))
        );

        // Mostra a árvore da versão atual
        ps.print(ps.current_version);
        println!("========================================");

        print!("Comando >> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim().to_lowercase();

        if input == "sair" {
            println!("Encerrando...");
            break;
        }

        // 1. Lógica de Visualização de Histórico (v <numero>)
        if input.starts_with('v') {
            let parts: Vec<&str> = input.split_whitespace().collect();
            if parts.len() > 1 {
                if let Ok(ver) = parts[1].parse::<u32>() {
                    if ps.roots.contains_key(&ver) {
                        println!("\n--- HISTÓRICO: Lendo Versão {} ---", ver);
                        ps.print(ver);
                        println!("-------------------------------");
                    } else {
                        println!("Erro: Versão {} não existe.", ver);
                    }
                }
            } else {
                println!("Uso: v <numero_da_versao>");
            }
            continue;
        }

        // 2. Lógica de Remoção (r <numero>)
        if input.starts_with('r') {
            let parts: Vec<&str> = input.split_whitespace().collect();
            if parts.len() > 1 {
                if let Ok(val) = parts[1].parse::<i32>() {
                    println!("Removendo valor {}...", val);
                    ps.remove(val);
                } else {
                    println!("Erro: Valor inválido para remoção.");
                }
            } else {
                println!("Uso: r <valor>");
            }
            continue;
        }

        // 3. Lógica de Inserção (apenas o número)
        match input.parse::<i32>() {
            Ok(val) => {
                println!("Inserindo valor {}...", val);
                ps.insert(val);
            }
            Err(_) => {
                if !input.is_empty() {
                    println!(
                        "Comando inválido. Use números para inserir, 'r' para remover ou 'v' para histórico."
                    );
                }
            }
        }
    }
}
