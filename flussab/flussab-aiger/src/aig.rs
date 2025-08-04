use std::{borrow::Cow, collections::hash_map, hash::Hash};

use thiserror::Error;
use zwohash::HashMap;
use crate::traversal::Graph;
use crate::Lit;
use std::collections::HashSet;
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Latch<L> {
    pub state: L,
    pub next_state: L,
    pub initialization: Option<bool>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct AndGate<L> {
    pub inputs: [L; 2],
    pub output: L,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum SymbolTarget {
    Input(usize),
    Output(usize),
    Latch(usize),
    BadStateProperty(usize),
    InvariantConstraint(usize),
    JusticeProperty(usize),
    FairnessConstraint(usize),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Symbol<'a> {
    pub target: SymbolTarget,
    pub name: Cow<'a, str>,
}

impl Symbol<'_> {
    pub fn into_owned_name(self) -> Symbol<'static> {
        Symbol {
            target: self.target,
            name: Cow::Owned(self.name.into_owned()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Aig<L> {
    pub max_var_index: usize,
    pub inputs: Vec<L>,
    pub latches: Vec<Latch<L>>,
    pub outputs: Vec<L>,
    pub bad_state_properties: Vec<L>,
    pub invariant_constraints: Vec<L>,
    pub justice_properties: Vec<Vec<L>>,
    pub fairness_constraints: Vec<L>,
    pub and_gates: Vec<AndGate<L>>,
    pub symbols: Vec<Symbol<'static>>,
    pub comment: Option<String>,
}
impl<L: Lit + std::cmp::Ord>  Aig<L> {
    /// 提取 cone，从指定的 `and_gate` 开始，追溯到所有的相关节点
    pub fn extract_cone(&self, gate_output: L) -> Self {
        // 存储新的 and_gates 和 inputs
        let mut new_and_gates = Vec::new();
        let mut new_inputs = HashSet::new();

        // 用于追踪访问过的节点
        let mut visited = HashSet::new();
        // 用于递归追踪的队列，起点是给定的 and_gate 的输出
        let mut queue = vec![gate_output];

        // 开始追踪逻辑锥
        while let Some(current_output) = queue.pop() {
            // 防止重复追踪
            if !visited.insert(current_output) {
                continue;
            }

            // 在 `and_gates` 中查找输出是 `current_output` 的 gate
            if let Some((_, and_gate)) = self.and_gates.iter().enumerate().find(|(_, gate)| gate.output == current_output) {
                // 记录当前 and_gate
                new_and_gates.push(and_gate.clone());

                // 遍历 and_gate 的 inputs
                for &input in &and_gate.inputs {
                    // 将 `L` 转换为 `u32` 以处理奇偶性
                    let input_code = input.code();

                    let normalized_input_code = if input_code % 2 == 1 {
                        input_code - 1
                    } else {
                        input_code
                    };

                    // 将 `u32` 转回 `L`
                    let normalized_input = L::from_code(normalized_input_code);

                    if self.inputs.contains(&normalized_input) {
                        // 如果 normalized input 是原始 AIG 的 primary input，记录为新 inputs
                        new_inputs.insert(normalized_input);
                    } else {
                        // 如果 normalized input 是另一个 and_gate 的输出，继续追踪
                        queue.push(normalized_input);
                    }
                }
            }
        }

        // 构建新的 inputs 和 symbols 的映射
        let mut new_inputs_vec: Vec<L> = new_inputs.into_iter().collect();
        new_inputs_vec.sort(); // 确保顺序一致

        // 构建新的 symbols
        let old_input_to_symbol: HashMap<_, _> = self
            .symbols
            .iter()
            .filter(|symbol| matches!(symbol.target, SymbolTarget::Input(_)))
            .map(|symbol| {
                if let SymbolTarget::Input(index) = symbol.target {
                    (self.inputs[index], symbol.clone())
                } else {
                    unreachable!()
                }
            })
            .collect();

        let mut new_symbols: Vec<_> = new_inputs_vec
            .iter()
            .enumerate()
            .map(|(index, &input)| {
                let old_symbol = old_input_to_symbol.get(&input).expect("Input not found in symbols");
                Symbol {
                    target: SymbolTarget::Input(index),
                    name: old_symbol.name.clone(),
                }
            })
            .collect();

            let symbol = Symbol {
                target: SymbolTarget::Output(0),
                name: Cow::Owned(format!("po{}", 0)),
            };
            new_symbols.push(symbol);
        

        // 构建新的 AIG
        Aig {
            max_var_index: self.max_var_index,
            inputs: new_inputs_vec,
            latches: vec![], // 假设逻辑锥中没有 latches
            outputs: vec![gate_output], // 输出是指定节点的 output
            bad_state_properties: vec![],
            invariant_constraints: vec![],
            justice_properties: vec![],
            fairness_constraints: vec![],
            and_gates: new_and_gates,
            symbols: new_symbols,
            comment: None,
        }
    }
}



   

impl<L: Lit> Aig<L> {
    pub fn from_graph(graph: &Graph) -> Self {
        let mut aig = Aig::<L>::default();
    
        // Step 1: 构建 inputs
        let mut nodes: Vec<_> = graph.nodes.iter().collect();
        nodes.sort_by_key(|(_, node)| node.order);
    
        for (id, node) in &nodes {
            if node.children.is_empty() && node.op != "n0"  {
                let input_lit = L::from_code(node.order.unwrap() * 2);
                aig.inputs.push(input_lit);
            }
        }
    
        // Step 2: 解析 root 节点
        let max_order = graph
            .nodes
            .values()
            .filter_map(|node| node.order)
            .max()
            .unwrap_or(0);
    
        for (i, root_id) in graph.root_eclasses.iter().enumerate() {
            if let Some(root_node) = graph.nodes.get(root_id) {
                let root_lit = if root_node.op == "!" {
                    L::from_code(max_order * 2 + 1)
                } else {
                    L::from_code(max_order * 2)
                };
                aig.outputs.push(root_lit);
    
                // 填充输出节点的 symbols
                // let symbol = Symbol {
                //     target: SymbolTarget::Output(i),
                //     name: Cow::Owned(format!("po{}", i)),
                // };
                // aig.symbols.push(symbol);
            }
        }
    
        // Step 3: 构建 and_gates
        for (id, node) in &nodes {
            if node.op == "*" {
                let output_lit = match node.order {
                    Some(order) => L::from_code(order * 2),
                    None => {
                        println!("Error: Node ID `{}` has no order", id);
                        panic!("Node {} has no order", id);
                    }
                };
        
                // Debugging child1
                let input_1 = if let Some(child1_id) = node.children.get(0) {
                    if let Some(child1_node) = graph.nodes.get(child1_id) {
                        if child1_node.op == "!" {
                            let grandchild_id = &child1_node.children[0];
                            if let Some(grandchild_node) = graph.nodes.get(grandchild_id) {
                                L::from_code(grandchild_node.order.unwrap_or_else(|| {
                                    println!(
                                        "Error: Grandchild node `{}` has no order (child of `{}`)",
                                        grandchild_id, child1_id
                                    );
                                    panic!("Grandchild node has no order: {}", grandchild_id);
                                }) * 2 + 1)
                            } else {
                                println!(
                                    "Error: Grandchild node not found for grandchild_id `{}` (child of `{}`)",
                                    grandchild_id, child1_id
                                );
                                panic!("Grandchild node not found: {}", grandchild_id);
                            }
                        } else {
                            L::from_code(child1_node.order.unwrap_or_else(|| {
                                println!(
                                    "Error: Child1 node `{}` has no order (child of `{}`)",
                                    child1_id, id
                                );
                                panic!("Child1 node has no order: {}", child1_id);
                            }) * 2)
                        }
                    } else {
                        println!("Error: Child1 node not found for child1_id `{}` (parent `{}`)", child1_id, id);
                        panic!("Child1 node not found: {}", child1_id);
                    }
                } else {
                    println!("Error: Node `{}` has no child1", id);
                    panic!("Node {} has no children1", id);
                };
        
                // Debugging child2
                let input_2 = if let Some(child2_id) = node.children.get(1) {
                    if let Some(child2_node) = graph.nodes.get(child2_id) {
                        if child2_node.op == "!" {
                            let grandchild_id = &child2_node.children[0];
                            if let Some(grandchild_node) = graph.nodes.get(grandchild_id) {
                                L::from_code(grandchild_node.order.unwrap_or_else(|| {
                                    println!(
                                        "Error: Grandchild node `{}` has no order (child of `{}`)",
                                        grandchild_id, child2_id
                                    );
                                    panic!("Grandchild node has no order: {}", grandchild_id);
                                }) * 2 + 1)
                            } else {
                                println!(
                                    "Error: Grandchild node not found for grandchild_id `{}` (child of `{}`)",
                                    grandchild_id, child2_id
                                );
                                panic!("Grandchild node not found: {}", grandchild_id);
                            }
                        } else {
                            L::from_code(child2_node.order.unwrap_or_else(|| {
                                println!(
                                    "Error: Child2 node `{}` has no order (child of `{}`)",
                                    child2_id, id
                                );
                                panic!("Child2 node has no order: {}", child2_id);
                            }) * 2)
                        }
                    } else {
                        println!("Error: Child2 node not found for child2_id `{}` (parent `{}`)", child2_id, id);
                        panic!("Child2 node not found: {}", child2_id);
                    }
                } else {
                    println!("Error: Node `{}` has no child2", id);
                    panic!("Node {} has no children2", id);
                };
        
                aig.and_gates.push(AndGate {
                    inputs: [input_1, input_2],
                    output: output_lit,
                });
            }
        }
        // println!("AIG And Gates:");
        // for (i, gate) in aig.and_gates.iter().enumerate() {
        //     println!(
        //         "Gate {}: Inputs = [{:?}, {:?}], Output = {:?}",
        //         i, gate.inputs[0], gate.inputs[1], gate.output
        //     );
        // }
        // Step 4: 填充 symbols 
        let mut symbols = Vec::new();
        for (id, node) in &nodes {
            if node.children.is_empty() && node.op != "n0" {
                let symbol = Symbol {
                    target: SymbolTarget::Input(node.order.unwrap() - 1),
                    name: Cow::Owned(node.op.clone()),
                };
                symbols.push(symbol);
            }
        }
        for (i, root_id) in graph.root_eclasses.iter().enumerate() {
            if let Some(root_node) = graph.nodes.get(root_id) {
                // SymbolTarget::Output，name 为 "po0"
                let symbol = Symbol {
                    target: SymbolTarget::Output(i),
                    name: Cow::Owned(format!("po{}", i)),
                };
                symbols.push(symbol);
            }
        }

        aig.symbols = symbols;
    
        aig
    }
    }

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum LitDef<L> {
    Constant,
    Input(usize),
    AndGate(OrderedAndGate<L>),
}

#[derive(Error, Debug)]
pub enum AigStructureError<L> {
    #[error("literal {lit} redefined")]
    LitAlreadyDefined { lit: L },
    #[error("used literal {lit} not defined")]
    LitNotDefined { lit: L },
    #[error("literal {lit} appears in a cycle")]
    FoundCycle { lit: L },
}

impl<L: Lit> Aig<L> {
    pub fn lit_defs(&self) -> Result<HashMap<L, LitDef<L>>, AigStructureError<L>> {
        let mut defs = <HashMap<L, LitDef<L>>>::default();

        defs.insert(L::from_code(0), LitDef::Constant);

        for (i, &lit) in self.inputs.iter().enumerate() {
            if defs.contains_key(&L::from_code(1 ^ lit.code()))
                || defs.insert(lit, LitDef::Input(i)).is_some()
            {
                return Err(AigStructureError::LitAlreadyDefined { lit });
            }
        }

        for &AndGate {
            inputs,
            output: lit,
        } in &self.and_gates
        {
            if defs.contains_key(&L::from_code(1 ^ lit.code()))
                || defs
                    .insert(lit, LitDef::AndGate(OrderedAndGate { inputs }))
                    .is_some()
            {
                return Err(AigStructureError::LitAlreadyDefined { lit });
            }
        }

        Ok(defs)
    }
}

pub struct Renumber<L: Lit> {
    config: RenumberConfig,
    defs: HashMap<L, LitDef<L>>,
    lit_map: LitMap<L>,
    last_code: usize,
    stack: Vec<Continuation<L>>,
    and_gates: Vec<OrderedAndGate<L>>,
    and_gate_index: HashMap<OrderedAndGate<L>, L>,
}

#[derive(Default)]
#[non_exhaustive]
pub struct RenumberConfig {
    pub trim: bool,
    pub structural_hash: bool,
    pub const_fold: bool,
}

impl RenumberConfig {
    pub fn trim(mut self, value: bool) -> Self {
        self.trim = value;
        self
    }

    pub fn structural_hash(mut self, value: bool) -> Self {
        self.structural_hash = value;
        self
    }

    pub fn const_fold(mut self, value: bool) -> Self {
        self.const_fold = value;
        self
    }
}

#[derive(Debug)]
enum State<L: Lit> {
    Transfer {
        lit: L,
    },
    Input0 {
        lit: L,
        def: AndGate<L>,
        transferred: L,
    },
    Input1 {
        lit: L,
        def: AndGate<L>,
        transferred: L,
    },
    Return {
        transferred: L,
    },
}

#[derive(Debug)]
enum Continuation<L: Lit> {
    Input0 { lit: L, def: AndGate<L> },
    Input1 { lit: L, def: AndGate<L> },
}

impl<L: Lit> Continuation<L> {
    fn returning(self, transferred: L) -> State<L> {
        match self {
            Continuation::Input0 { lit, def } => State::Input0 {
                lit,
                def,
                transferred,
            },
            Continuation::Input1 { lit, def } => State::Input1 {
                lit,
                def,
                transferred,
            },
        }
    }
}

impl<L: Lit> Renumber<L> {
    pub fn new(config: RenumberConfig, aig: &Aig<L>) -> Result<Self, AigStructureError<L>> {
        let defs: HashMap<L, LitDef<L>> = aig.lit_defs()?;
        let mut new = Self {
            config,
            defs,
            lit_map: Default::default(),
            last_code: 0,
            stack: vec![],
            and_gates: vec![],
            and_gate_index: Default::default(),
        };

        new.initialize(aig)?;
        Ok(new)
    }

    fn initialize(&mut self, aig: &Aig<L>) -> Result<(), AigStructureError<L>> {
        self.lit_map.insert(L::from_code(0), L::from_code(0));

        for &lit in &aig.inputs {
            self.last_code += 2;
            self.lit_map.insert(lit, L::from_code(self.last_code));
        }

        for latch in &aig.latches {
            self.last_code += 2;
            self.lit_map
                .insert(latch.state, L::from_code(self.last_code));
        }

        if !self.config.trim {
            for and in &aig.and_gates {
                self.transfer(and.output)?;
            }
        }

        for latch in &aig.latches {
            self.transfer(latch.next_state)?;
        }

        for lits in [
            &aig.outputs,
            &aig.bad_state_properties,
            &aig.invariant_constraints,
            &aig.fairness_constraints,
        ] {
            for &lit in lits {
                self.transfer(lit)?;
            }
        }

        for lits in &aig.justice_properties {
            for &lit in lits {
                self.transfer(lit)?;
            }
        }

        Ok(())
    }

    pub fn renumber_aig(
        config: RenumberConfig,
        aig: &Aig<L>,
    ) -> Result<(OrderedAig<L>, Self), AigStructureError<L>> {
        let mut new = Self::new(config, aig)?;
        let ordered_aig = OrderedAig {
            max_var_index: new.last_code >> 1,
            input_count: aig.inputs.len(),
            latches: aig
                .latches
                .iter()
                .map(
                    |&Latch {
                         next_state,
                         initialization,
                         ..
                     }| OrderedLatch {
                        next_state: new.lit_map.get(next_state).unwrap(),
                        initialization,
                    },
                )
                .collect(),
            outputs: aig
                .outputs
                .iter()
                .map(|&lit| new.lit_map.get(lit).unwrap())
                .collect(),
            bad_state_properties: aig
                .bad_state_properties
                .iter()
                .map(|&lit| new.lit_map.get(lit).unwrap())
                .collect(),
            invariant_constraints: aig
                .invariant_constraints
                .iter()
                .map(|&lit| new.lit_map.get(lit).unwrap())
                .collect(),
            justice_properties: aig
                .justice_properties
                .iter()
                .map(|lits| {
                    lits.iter()
                        .map(|&lit| new.lit_map.get(lit).unwrap())
                        .collect()
                })
                .collect(),
            fairness_constraints: aig
                .fairness_constraints
                .iter()
                .map(|&lit| new.lit_map.get(lit).unwrap())
                .collect(),
            and_gates: std::mem::take(&mut new.and_gates),
            symbols: aig.symbols.clone(),
            comment: aig.comment.clone(),
        };

        Ok((ordered_aig, new))
    }

    fn transfer(&mut self, lit: L) -> Result<L, AigStructureError<L>> {
        let mut state = State::Transfer { lit };
        'outer: loop {
            match state {
                State::Transfer { lit } => {
                    if let Some(transferred) = self.lit_map.get(lit) {
                        state = State::Return { transferred };
                        continue 'outer;
                    }
                    match self.stack.get(self.stack.len() / 2) {
                        Some(
                            &Continuation::Input0 { lit: l, .. }
                            | &Continuation::Input1 { lit: l, .. },
                        ) if lit == l => return Err(AigStructureError::FoundCycle { lit }),
                        _ => (),
                    };

                    let mut def = None;

                    for output in [lit, L::from_code(1 ^ lit.code())] {
                        if let Some(&LitDef::AndGate(OrderedAndGate { inputs })) =
                            self.defs.get(&output)
                        {
                            def = Some(AndGate { inputs, output });
                        }
                    }

                    let def = if let Some(def) = def {
                        def
                    } else {
                        return Err(AigStructureError::LitNotDefined { lit });
                    };

                    self.stack.push(Continuation::Input0 { lit, def });
                    state = State::Transfer { lit: def.inputs[0] };
                    continue 'outer;
                }
                State::Input0 {
                    lit,
                    mut def,
                    transferred,
                } => {
                    def.inputs[0] = transferred;

                    self.stack.push(Continuation::Input1 { lit, def });
                    state = State::Transfer { lit: def.inputs[1] };
                    continue 'outer;
                }
                State::Input1 {
                    lit,
                    mut def,
                    transferred,
                } => {
                    def.inputs[1] = transferred;

                    def.inputs.sort_unstable_by_key(|input| !input.code());

                    let and_gate = OrderedAndGate { inputs: def.inputs };

                    let new_lit;
                    let new_code;

                    if self.config.const_fold {
                        let codes = def.inputs.map(|input| input.code());
                        let mut folded = None;

                        if codes[0] == 0 || codes[1] == 0 {
                            folded = Some(L::from_code(0));
                        } else if codes[0] == 1 || codes[0] == codes[1] {
                            folded = Some(def.inputs[1]);
                        } else if codes[1] == 1 {
                            folded = Some(def.inputs[0]);
                        }

                        if let Some(folded) = folded {
                            self.lit_map.insert(def.output, folded);
                            state = State::Return {
                                transferred: L::from_code(
                                    folded.code() ^ lit.code() ^ def.output.code(),
                                ),
                            };
                            continue 'outer;
                        }
                    }

                    if self.config.structural_hash {
                        match self.and_gate_index.entry(and_gate) {
                            hash_map::Entry::Occupied(entry) => {
                                new_lit = *entry.get();
                                new_code = new_lit.code();
                            }
                            hash_map::Entry::Vacant(entry) => {
                                self.last_code += 2;
                                new_code = self.last_code;
                                new_lit = L::from_code(new_code);
                                entry.insert(new_lit);
                                self.and_gates.push(OrderedAndGate { inputs: def.inputs });
                            }
                        }
                    } else {
                        self.last_code += 2;
                        new_code = self.last_code;
                        new_lit = L::from_code(new_code);
                        self.and_gates.push(OrderedAndGate { inputs: def.inputs });
                    }

                    self.lit_map.insert(def.output, new_lit);

                    state = State::Return {
                        transferred: L::from_code(new_code ^ lit.code() ^ def.output.code()),
                    };
                    continue 'outer;
                }
                State::Return { transferred } => match self.stack.pop() {
                    Some(continuation) => {
                        state = continuation.returning(transferred);
                        continue 'outer;
                    }
                    None => return Ok(transferred),
                },
            }
        }
    }

    pub fn lit_map(&self) -> &LitMap<L> {
        &self.lit_map
    }

    pub fn and_gates(&self) -> &[OrderedAndGate<L>] {
        &self.and_gates
    }
}

pub struct LitMap<L: Lit> {
    map: HashMap<L, L>,
}

impl<L: Lit> LitMap<L> {
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn contains_key(&self, lit: L) -> bool {
        self.map.contains_key(&L::from_code(lit.code() & !1))
    }
}

impl<L: Lit> Default for LitMap<L> {
    fn default() -> Self {
        Self {
            map: Default::default(),
        }
    }
}

impl<L: Lit + std::fmt::Debug + std::fmt::Debug> std::fmt::Debug for LitMap<L> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.map, f)
    }
}

impl<L: Lit> LitMap<L> {
    pub fn insert(&mut self, key: L, value: L) -> Option<L> {
        let key_code = key.code();
        let entry_key = L::from_code(key_code & !1);
        let entry_value = L::from_code(value.code() ^ (key_code & 1));
        self.map
            .insert(entry_key, entry_value)
            .map(|old| L::from_code(old.code() ^ (key_code & 1)))
    }

    pub fn get(&self, key: L) -> Option<L> {
        let key_code = key.code();
        let entry_key = L::from_code(key_code & !1);
        self.map
            .get(&entry_key)
            .map(|found| L::from_code(found.code() ^ (key_code & 1)))
    }
}

impl<L> Default for Aig<L> {
    fn default() -> Self {
        Self {
            max_var_index: 0,
            inputs: vec![],
            latches: vec![],
            outputs: vec![],
            bad_state_properties: vec![],
            invariant_constraints: vec![],
            justice_properties: vec![],
            fairness_constraints: vec![],
            and_gates: vec![],
            symbols: vec![],
            comment: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct OrderedLatch<L> {
    pub next_state: L,
    pub initialization: Option<bool>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct OrderedAndGate<L> {
    pub inputs: [L; 2],
}

#[derive(Clone, Debug)]
pub struct OrderedAig<L> {
    pub max_var_index: usize,
    pub input_count: usize,
    pub latches: Vec<OrderedLatch<L>>,
    pub outputs: Vec<L>,
    pub bad_state_properties: Vec<L>,
    pub invariant_constraints: Vec<L>,
    pub justice_properties: Vec<Vec<L>>,
    pub fairness_constraints: Vec<L>,
    pub and_gates: Vec<OrderedAndGate<L>>,
    pub symbols: Vec<Symbol<'static>>,
    pub comment: Option<String>,
}

impl<L: Lit> From<OrderedAig<L>> for Aig<L> {
    fn from(ordered: OrderedAig<L>) -> Self {
        let first_latch = 1 + ordered.input_count;
        let first_and = first_latch + ordered.latches.len();
        Aig {
            max_var_index: ordered.max_var_index,
            inputs: (0..ordered.input_count)
                .map(|i| L::from_code((i + 1) * 2))
                .collect(),
            latches: ordered
                .latches
                .into_iter()
                .enumerate()
                .map(
                    |(
                        i,
                        OrderedLatch {
                            next_state,
                            initialization,
                        },
                    )| Latch {
                        state: L::from_code((i + first_latch) * 2),
                        next_state,
                        initialization,
                    },
                )
                .collect(),
            outputs: ordered.outputs,
            bad_state_properties: ordered.bad_state_properties,
            invariant_constraints: ordered.invariant_constraints,
            justice_properties: ordered.justice_properties,
            fairness_constraints: ordered.fairness_constraints,
            and_gates: ordered
                .and_gates
                .into_iter()
                .enumerate()
                .map(|(i, OrderedAndGate { inputs })| AndGate {
                    inputs,
                    output: L::from_code((i + first_and) * 2),
                })
                .collect(),
            symbols: ordered.symbols,
            comment: ordered.comment,
        }
    }
}

impl<L> Default for OrderedAig<L> {
    fn default() -> Self {
        Self {
            max_var_index: 0,
            input_count: 0,
            latches: vec![],
            outputs: vec![],
            bad_state_properties: vec![],
            invariant_constraints: vec![],
            justice_properties: vec![],
            fairness_constraints: vec![],
            and_gates: vec![],
            symbols: vec![],
            comment: None,
        }
    }
}
