use proconio::input;
use std::time::Instant;
use std::collections::VecDeque;
use rand::{rngs::StdRng, SeedableRng};
#[cfg(debug_assertions)]
use std::io::Write;

const FALLBACK_STALL_LIMIT: usize = 3;
const SAFE_COLLECT_TIME_LIMIT_MS: u128 = 1900;
const SAFE_COLLECT_TURN_LIMIT: usize = 10000;
const SAFE_BRANCH_ENDGAME_REMAINING_FOOD_LIMIT: usize = 24;
const SAFE_BRANCH_MIN_SAFE_COLLECT_COUNT: usize = 2;
const GREEDY_TARGET_CANDIDATE_LIMIT: usize = 3;
const GREEDY_FALLBACK_CANDIDATE_LIMIT: usize = 3;
const TARGET_BFS_ORDERS: [[char; 4]; 4] = [
    ['U', 'D', 'L', 'R'],
    ['R', 'D', 'L', 'U'],
    ['L', 'U', 'R', 'D'],
    ['D', 'R', 'U', 'L'],
];

fn read_input() -> Input {
    input! {
        n: usize,
        m: usize,
        c: usize,
        d: [usize; m],
        f: [[usize; n]; n],
    }

    Input { n, m, c, d, f }
}

#[derive(Clone, Debug)]
struct Input {
    n: usize,
    m: usize,
    c: usize,
    d: Vec<usize>,
    f: Vec<Vec<usize>>,
}

struct Solver {
    rng: StdRng,
    start: Instant,
    input: Input,
    ops: Vec<char>,
    best_snapshot: Option<OutputSnapshot>,
    last_progress: Option<ProgressSnapshot>,
    safe_collect_active: bool,
    force_safe_collect_mode: bool,
    escape_bite_count: usize,
    rebuild_bite_count: usize,
    safe_collect_count: usize,
    previous_phase: Option<Phase>,
    stop_reason: &'static str,
}

struct SnakeState {
    board: Vec<Vec<usize>>,
    positions: Vec<(usize, usize)>,
    colors: Vec<usize>,
}

#[derive(Clone, Debug)]
struct OutputSnapshot {
    ops: Vec<char>,
    score: usize,
    stats: OutputStats,
}

#[derive(Clone, Debug, Default)]
struct OutputStats {
    escape_bite_count: usize,
    rebuild_bite_count: usize,
    safe_collect_count: usize,
    forced_safe_collect: bool,
    used_safe_branch: bool,
    stop_reason: &'static str,
}

#[derive(Clone, Debug)]
struct BfsResult {
    start: (usize, usize),
    reachable: Vec<Vec<bool>>,
    dist: Vec<Vec<Option<usize>>>,
    parent: Vec<Vec<Option<(usize, usize)>>>,
    parent_move: Vec<Vec<Option<char>>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FoodTarget {
    cell: (usize, usize),
    dist: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GreedyTargetEval {
    consecutive_target_hits_3: usize,
    consecutive_target_hits: usize,
    next_target_dist: Option<usize>,
    next_fallback_dist: Option<usize>,
    reachable_cell_count: usize,
    reachable_food_count: usize,
    prefix_len: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SafeCollectBiteEval {
    followup_food_collected: bool,
    projected_prefix_len: usize,
    prefix_len: usize,
    next_target_dist: Option<usize>,
    next_fallback_dist: Option<usize>,
    reachable_cell_count: usize,
    reachable_food_count: usize,
    bitten_length: usize,
    followup_len: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SafeCollectRouteEval {
    projected_prefix_len: usize,
    prefix_len: usize,
    next_target_dist: Option<usize>,
    next_fallback_dist: Option<usize>,
    reachable_cell_count: usize,
    reachable_food_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    GreedyTarget,
    GreedyFallback,
    SafeCollect,
    BiteRebuild,
}

impl Phase {
    fn as_str(self) -> &'static str {
        match self {
            Phase::GreedyTarget => "GreedyTarget",
            Phase::GreedyFallback => "GreedyFallback",
            Phase::SafeCollect => "SafeCollect",
            Phase::BiteRebuild => "BiteRebuild",
        }
    }
}

#[derive(Clone, Debug)]
struct Plan {
    phase: Phase,
    moves: Vec<char>,
    resume_greedy_after_apply: bool,
}

#[derive(Clone, Copy, Debug)]
struct ProgressSnapshot {
    prefix_len: usize,
    remaining_food_count: usize,
    can_reach_any_food: bool,
    fallback_stall_count: usize,
    is_stalled: bool,
}

impl Solver {
    fn new(seed: u64, start: Instant, input: Input) -> Self {
        let rng = StdRng::seed_from_u64(seed);

        Self {
            rng,
            start,
            input,
            ops: Vec::new(),
            best_snapshot: None,
            last_progress: None,
            safe_collect_active: false,
            force_safe_collect_mode: false,
            escape_bite_count: 0,
            rebuild_bite_count: 0,
            safe_collect_count: 0,
            previous_phase: None,
            stop_reason: "unknown",
        }
    }

    fn solve(&mut self) {
        self.ops.clear();
        self.best_snapshot = None;
        let mut state = self.initial_state();
        self.last_progress = None;
        self.safe_collect_active = false;
        self.force_safe_collect_mode = false;
        self.escape_bite_count = 0;
        self.rebuild_bite_count = 0;
        self.safe_collect_count = 0;
        self.previous_phase = None;
        self.stop_reason = "unknown";
        self.reset_phase_trace();
        self.reset_debug_answer();

        loop {
            if self.should_stop(&state) {
                self.stop_reason = if self.force_safe_collect_mode && state.colors.len() == self.input.m {
                    "force_safe_full_length"
                } else {
                    "completed"
                };
                break;
            }
            if self.ops.len() >= 100000 {
                self.stop_reason = "turn_limit";
                break;
            }

            let phase = self.choose_phase(&state);
            self.write_phase_trace(self.ops.len(), phase);
            let plan = match phase {
                Phase::GreedyTarget => self.plan_greedy_target(&state),
                Phase::GreedyFallback => self.plan_greedy_fallback(&state),
                Phase::SafeCollect => self.plan_safe_collect(&state),
                Phase::BiteRebuild => self.plan_bite_rebuild(&state),
            };
            let Some(plan) = plan else {
                let mut stats = self.current_output_stats(false);
                stats.stop_reason = "no_plan_snapshot";
                self.update_best_snapshot(&self.ops.clone(), stats);
                self.stop_reason = "no_plan";
                break;
            };

            if plan.moves.is_empty() {
                self.stop_reason = "empty_plan";
                break;
            }

            if self.should_launch_safe_branch(&state, &plan) {
                self.run_safe_branch(&state);
            }

            if plan.phase == Phase::SafeCollect && self.previous_phase != Some(Phase::SafeCollect) {
                self.safe_collect_count += 1;
            }

            let length_before = state.colors.len();
            let prefix_before = self.prefix_len(&state);
            self.apply_moves(&mut state, &plan.moves);
            if state.colors.len() < length_before {
                if plan.phase == Phase::BiteRebuild {
                    self.rebuild_bite_count += 1;
                } else if plan.phase == Phase::SafeCollect {
                    self.escape_bite_count += 1;
                }
            }
            self.ops.extend(plan.moves.iter().copied());
            let prefix_after = self.prefix_len(&state);
            if prefix_after > prefix_before {
                let mut stats = self.current_output_stats(false);
                stats.stop_reason = if prefix_after == self.input.m {
                    "completed"
                } else {
                    "main_prefix_snapshot"
                };
                self.update_best_snapshot(&self.ops.clone(), stats);
            }
            if length_before < self.input.m && state.colors.len() == self.input.m {
                let mut stats = self.current_output_stats(false);
                stats.stop_reason = if self.prefix_len(&state) == self.input.m {
                    "completed"
                } else {
                    "main_full_length_snapshot"
                };
                self.update_best_snapshot(&self.ops.clone(), stats);
            }
            self.update_progress(&state, plan.phase, plan.resume_greedy_after_apply);
            self.previous_phase = Some(plan.phase);
        }

        let final_ops = self.ops.clone();
        self.update_best_snapshot(&final_ops, self.current_output_stats(false));
        if let Some(best) = &self.best_snapshot {
            self.ops = best.ops.clone();
        }
        self.write_debug_answer();
    }

    fn ans(&self) {
        for &op in &self.ops {
            println!("{op}");
        }
    }

    fn score(&self) -> usize {
        let state = self.simulate();
        self.absolute_score(&state, self.ops.len())
    }

    fn result(&self) {
        let state = self.simulate();
        let k = state.colors.len();
        let m = self.input.m;
        let e = state
            .colors
            .iter()
            .zip(self.input.d.iter())
            .filter(|(actual, desired)| actual != desired)
            .count();
        let t = self.ops.len();
        let prefix_len = self.prefix_len(&state);
        let remaining_food = self.remaining_food_count(&state);
        let score = self.absolute_score(&state, t);
        let stats = self
            .best_snapshot
            .as_ref()
            .map(|snapshot| snapshot.stats.clone())
            .unwrap_or_else(|| self.current_output_stats(false));

        eprintln!(
            "{{ \"score\": {}, \"k\": {}, \"m\": {}, \"e\": {}, \"t\": {}, \"prefix_len\": {}, \"remaining_food\": {}, \"completed\": {}, \"full_length\": {}, \"escape_bite_count\": {}, \"rebuild_bite_count\": {}, \"safe_collect_count\": {}, \"forced_safe_collect\": {}, \"used_safe_branch\": {}, \"stop_reason\": \"{}\", \"elapsed_ms\": {} }}",
            score,
            k,
            m,
            e,
            t,
            prefix_len,
            remaining_food,
            k == m && e == 0,
            k == m,
            stats.escape_bite_count,
            stats.rebuild_bite_count,
            stats.safe_collect_count,
            stats.forced_safe_collect,
            stats.used_safe_branch,
            stats.stop_reason,
            self.start.elapsed().as_millis(),
        );
    }

    fn build_zigzag_path(&self) -> Vec<(usize, usize)> {
        let mut path = Vec::with_capacity(self.input.n * self.input.n);

        for j in 0..self.input.n {
            if j % 2 == 0 {
                for i in 0..self.input.n {
                    path.push((i, j));
                }
            } else {
                for i in (0..self.input.n).rev() {
                    path.push((i, j));
                }
            }
        }

        path
    }

    fn initial_state(&self) -> SnakeState {
        SnakeState {
            board: self.input.f.clone(),
            positions: vec![(4, 0), (3, 0), (2, 0), (1, 0), (0, 0)],
            colors: vec![1; 5],
        }
    }

    fn step(from: (usize, usize), to: (usize, usize)) -> char {
        match (
            to.0 as isize - from.0 as isize,
            to.1 as isize - from.1 as isize,
        ) {
            (-1, 0) => 'U',
            (1, 0) => 'D',
            (0, -1) => 'L',
            (0, 1) => 'R',
            _ => panic!("path must move to an adjacent cell"),
        }
    }

    fn simulate(&self) -> SnakeState {
        self.simulate_ops(&self.ops)
    }

    fn simulate_ops(&self, ops: &[char]) -> SnakeState {
        let mut state = self.initial_state();

        self.apply_moves(&mut state, ops);

        state
    }

    fn score_ops(&self, ops: &[char]) -> usize {
        let state = self.simulate_ops(ops);
        self.absolute_score(&state, ops.len())
    }

    fn absolute_score(&self, state: &SnakeState, turn_count: usize) -> usize {
        let k = state.colors.len();
        let e = state
            .colors
            .iter()
            .zip(self.input.d.iter())
            .filter(|(actual, desired)| actual != desired)
            .count();

        turn_count + 10000 * (e + 2 * (self.input.m - k))
    }

    fn current_output_stats(&self, used_safe_branch: bool) -> OutputStats {
        OutputStats {
            escape_bite_count: self.escape_bite_count,
            rebuild_bite_count: self.rebuild_bite_count,
            safe_collect_count: self.safe_collect_count,
            forced_safe_collect: self.force_safe_collect_mode,
            used_safe_branch,
            stop_reason: self.stop_reason,
        }
    }

    fn update_best_snapshot(&mut self, ops: &[char], stats: OutputStats) {
        let candidate = OutputSnapshot {
            ops: ops.to_vec(),
            score: self.score_ops(ops),
            stats,
        };
        if self
            .best_snapshot
            .as_ref()
            .is_none_or(|best| {
                candidate.score < best.score
                    || (candidate.score == best.score && candidate.ops.len() < best.ops.len())
                    || (candidate.score == best.score
                        && candidate.ops.len() == best.ops.len()
                        && candidate.stats.stop_reason == "completed"
                        && best.stats.stop_reason != "completed")
            })
        {
            self.best_snapshot = Some(candidate);
        }
    }

    fn run_safe_branch(&mut self, state: &SnakeState) {
        let Some((branch_ops, branch_stats)) = self.build_safe_branch_ops(state) else {
            return;
        };
        self.update_best_snapshot(&branch_ops, branch_stats);
    }

    fn build_safe_branch_ops(&self, state: &SnakeState) -> Option<(Vec<char>, OutputStats)> {
        let mut branch_state = SnakeState {
            board: state.board.clone(),
            positions: state.positions.clone(),
            colors: state.colors.clone(),
        };
        let mut branch_ops = self.ops.clone();
        let mut branch_escape_bite_count = 0;

        while branch_state.colors.len() < self.input.m && branch_ops.len() < 100000 {
            let moves = if let Some(moves) = self.plan_zigzag_safe_collect_moves(&branch_state) {
                moves
            } else if let Some(moves) = self.plan_forced_safe_collect_bite(&branch_state) {
                branch_escape_bite_count += 1;
                moves
            } else {
                return None;
            };
            if moves.is_empty() {
                return None;
            }

            self.apply_moves(&mut branch_state, &moves);
            branch_ops.extend(moves);
        }

        if branch_state.colors.len() == self.input.m {
            Some((
                branch_ops,
                OutputStats {
                    escape_bite_count: branch_escape_bite_count,
                    rebuild_bite_count: 0,
                    safe_collect_count: 1,
                    forced_safe_collect: false,
                    used_safe_branch: true,
                    stop_reason: "safe_branch_full_length",
                },
            ))
        } else {
            None
        }
    }

    fn should_stop(&self, state: &SnakeState) -> bool {
        if self.force_safe_collect_mode && state.colors.len() == self.input.m {
            return true;
        }
        state.colors.len() == self.input.m && self.prefix_len(state) == self.input.m
    }

    fn choose_phase(&mut self, state: &SnakeState) -> Phase {
        if self.should_force_safe_collect() {
            self.force_safe_collect_mode = true;
            self.safe_collect_active = true;
            return Phase::SafeCollect;
        }
        if state.colors.len() == self.input.m && self.prefix_len(state) < self.input.m {
            return Phase::BiteRebuild;
        }
        if self.safe_collect_active {
            self.safe_collect_active = true;
            return Phase::SafeCollect;
        }

        if self.plan_greedy_target_inner(state).is_some() {
            return Phase::GreedyTarget;
        }

        if self.plan_greedy_fallback_inner(state).is_some() {
            return Phase::GreedyFallback;
        }

        if !self.can_reach_any_food(state) {
            return Phase::SafeCollect;
        }
        Phase::GreedyFallback
    }

    fn plan_greedy_target(&self, state: &SnakeState) -> Option<Plan> {
        let (plan, bfs, target, target_color) = self.plan_greedy_target_inner(state)?;
        self.write_bfs_snapshot(
            self.ops.len(),
            Phase::GreedyTarget,
            Some(target_color),
            target.cell,
            &bfs,
        );
        Some(plan)
    }

    fn plan_greedy_fallback(&self, state: &SnakeState) -> Option<Plan> {
        let (plan, bfs, target) = self.plan_greedy_fallback_inner(state)?;
        self.write_bfs_snapshot(
            self.ops.len(),
            Phase::GreedyFallback,
            None,
            target.cell,
            &bfs,
        );
        Some(plan)
    }

    fn plan_safe_collect(&self, state: &SnakeState) -> Option<Plan> {
        let (moves, resume_greedy_after_apply) = if self.force_safe_collect_mode {
            let moves = self
                .plan_zigzag_safe_collect_moves(state)
                .or_else(|| self.plan_forced_safe_collect_bite(state))?;
            (moves, false)
        } else {
            let no_bite_retry = self.plan_pre_bite_no_bite_continuation(state);
            let bite_moves = self.plan_safe_collect_bfs_bite(state);
            let resume_greedy_after_apply = no_bite_retry.is_some() || bite_moves.is_some();
            let moves = no_bite_retry
                .or(bite_moves)
                .or_else(|| self.plan_zigzag_safe_collect_moves(state))
                .or_else(|| self.plan_greedy_fallback_inner(state).map(|(plan, _, _)| plan.moves))?;
            (moves, resume_greedy_after_apply)
        };
        Some(Plan {
            phase: Phase::SafeCollect,
            moves,
            resume_greedy_after_apply,
        })
    }

    fn plan_pre_bite_no_bite_continuation(&self, state: &SnakeState) -> Option<Vec<char>> {
        if let Some((plan, _, _, _)) = self.plan_greedy_target_inner(state) {
            let eval = self.evaluate_greedy_target_candidate(state, &plan.moves);
            if eval.consecutive_target_hits > 0
                || eval.next_target_dist.is_some()
                || eval.next_fallback_dist.is_some()
            {
                return Some(plan.moves);
            }
        }

        if let Some((plan, _, _)) = self.plan_greedy_fallback_inner(state) {
            let eval = self.evaluate_greedy_fallback_candidate(state, &plan.moves);
            if eval.0 || eval.2 {
                return Some(plan.moves);
            }
        }

        None
    }

    fn plan_bite_rebuild(&self, state: &SnakeState) -> Option<Plan> {
        let current_prefix_len = self.prefix_len(state);
        let mut best_strict: Option<(usize, usize, usize, Vec<char>)> = None;
        let mut best_relaxed: Option<(usize, usize, usize, Vec<char>)> = None;
        let mut best_any: Option<(usize, bool, bool, usize, usize, usize, usize, usize, Vec<char>)> = None;

        for idx in 2..state.positions.len().saturating_sub(1) {
            let Some((moves, bitten)) = self.simulate_bite_candidate(state, idx) else {
                continue;
            };
            let prefix_len = self.prefix_len(&bitten);
            let repaired_prefix_len = self.project_prefix_after_resume(&bitten);
            let reexpand_bfs = self.bfs_reachable_first_food(&bitten);
            let candidate = (repaired_prefix_len, prefix_len, bitten.colors.len(), moves);
            if prefix_len == bitten.colors.len()
                && best_strict.as_ref().is_none_or(|current| {
                candidate.0 > current.0
                    || (candidate.0 == current.0 && candidate.1 > current.1)
                    || (candidate.0 == current.0 && candidate.1 == current.1 && candidate.2 > current.2)
                    || (candidate.0 == current.0
                        && candidate.1 == current.1
                        && candidate.2 == current.2
                        && candidate.3.len() < current.3.len())
            })
            {
                best_strict = Some(candidate.clone());
            }
            if (repaired_prefix_len > current_prefix_len || prefix_len > current_prefix_len)
                && best_relaxed.as_ref().is_none_or(|current| {
                    candidate.0 > current.0
                        || (candidate.0 == current.0 && candidate.1 > current.1)
                        || (candidate.0 == current.0 && candidate.1 == current.1 && candidate.2 > current.2)
                        || (candidate.0 == current.0
                            && candidate.1 == current.1
                            && candidate.2 == current.2
                            && candidate.3.len() < current.3.len())
                })
            {
                best_relaxed = Some(candidate.clone());
            }
            let prefix_loss = current_prefix_len.saturating_sub(prefix_len);
            let can_resume_target = self.plan_greedy_target_inner(&bitten).is_some();
            let can_resume_fallback = self.plan_greedy_fallback_inner(&bitten).is_some();
            let any_candidate = (
                prefix_loss,
                can_resume_target,
                can_resume_fallback,
                prefix_len,
                reexpand_bfs.reachable_food_count(&bitten),
                reexpand_bfs.reachable_cell_count(),
                bitten.colors.len(),
                candidate.0,
                candidate.3.clone(),
            );
            if best_any.as_ref().is_none_or(|current| {
                any_candidate.0 < current.0
                    || (any_candidate.0 == current.0 && any_candidate.1 && !current.1)
                    || (any_candidate.0 == current.0
                        && any_candidate.1 == current.1
                        && any_candidate.2
                        && !current.2)
                    || (any_candidate.0 == current.0
                        && any_candidate.1 == current.1
                        && any_candidate.2 == current.2
                        && any_candidate.3 > current.3)
                    || (any_candidate.0 == current.0
                        && any_candidate.1 == current.1
                        && any_candidate.2 == current.2
                        && any_candidate.3 == current.3
                        && any_candidate.4 > current.4)
                    || (any_candidate.0 == current.0
                        && any_candidate.1 == current.1
                        && any_candidate.2 == current.2
                        && any_candidate.3 == current.3
                        && any_candidate.4 == current.4
                        && any_candidate.5 > current.5)
                    || (any_candidate.0 == current.0
                        && any_candidate.1 == current.1
                        && any_candidate.2 == current.2
                        && any_candidate.3 == current.3
                        && any_candidate.4 == current.4
                        && any_candidate.5 == current.5
                        && any_candidate.6 > current.6)
                    || (any_candidate.0 == current.0
                        && any_candidate.1 == current.1
                        && any_candidate.2 == current.2
                        && any_candidate.3 == current.3
                        && any_candidate.4 == current.4
                        && any_candidate.5 == current.5
                        && any_candidate.6 == current.6
                        && any_candidate.7 > current.7)
                    || (any_candidate.0 == current.0
                        && any_candidate.1 == current.1
                        && any_candidate.2 == current.2
                        && any_candidate.3 == current.3
                        && any_candidate.4 == current.4
                        && any_candidate.5 == current.5
                        && any_candidate.6 == current.6
                        && any_candidate.7 == current.7
                        && any_candidate.8.len() < current.8.len())
            }) {
                best_any = Some(any_candidate);
            }
        }

        let moves = if let Some((_, _, _, moves)) = best_strict.or(best_relaxed) {
            moves
        } else {
            let (_, _, _, _, _, _, _, _, moves) = best_any?;
            moves
        };
        Some(Plan {
            phase: Phase::BiteRebuild,
            moves,
            resume_greedy_after_apply: false,
        })
    }

    fn apply_moves(&self, state: &mut SnakeState, moves: &[char]) {
        for &op in moves {
            self.apply_move(state, op);
        }
    }

    fn plan_safe_collect_bfs_bite(&self, state: &SnakeState) -> Option<Vec<char>> {
        if self.can_reach_any_food(state) {
            return None;
        }

        self.plan_forced_safe_collect_bite(state)
    }

    fn plan_forced_safe_collect_bite(&self, state: &SnakeState) -> Option<Vec<char>> {
        let mut best: Option<(SafeCollectBiteEval, usize, Vec<char>)> = None;

        for idx in 2..state.positions.len().saturating_sub(1) {
            let Some((moves, bitten)) = self.simulate_bite_candidate(state, idx) else {
                continue;
            };
            let eval = self.evaluate_safe_collect_bite_candidate(&bitten);
            let candidate = (eval, moves.len(), moves);
            if best.as_ref().is_none_or(|current| {
                self.is_better_safe_collect_bite_candidate(
                    candidate.0,
                    candidate.1,
                    current.0,
                    current.1,
                )
            }) {
                best = Some(candidate);
            }
        }

        best.map(|(_, _, moves)| moves)
    }

    fn simulate_bite_candidate(
        &self,
        state: &SnakeState,
        idx: usize,
    ) -> Option<(Vec<char>, SnakeState)> {
        let target = *state.positions.get(idx)?;
        let bfs = self.bfs_reachable_body_target(state, target);
        let moves = bfs.restore_moves(target)?;
        let mut bitten = SnakeState {
            board: state.board.clone(),
            positions: state.positions.clone(),
            colors: state.colors.clone(),
        };
        self.apply_moves(&mut bitten, &moves);
        if bitten.colors.len() >= state.colors.len() {
            return None;
        }
        Some((moves, bitten))
    }

    fn evaluate_safe_collect_bite_candidate(&self, bitten: &SnakeState) -> SafeCollectBiteEval {
        let mut followed = SnakeState {
            board: bitten.board.clone(),
            positions: bitten.positions.clone(),
            colors: bitten.colors.clone(),
        };
        let zigzag_followup = self.plan_zigzag_safe_collect_moves(bitten);
        if let Some(moves) = zigzag_followup.as_ref() {
            self.apply_moves(&mut followed, moves);
        }

        let next_target_dist = if followed.colors.len() < self.input.m {
            let next_target_color = self.input.d[followed.colors.len()];
            let next_bfs = self.bfs_reachable_target_color(&followed, next_target_color);
            self.choose_nearest_food_of_color(&followed, &next_bfs, next_target_color)
                .map(|target| target.dist)
        } else {
            Some(0)
        };
        let next_fallback_dist = if followed.colors.len() < self.input.m {
            let next_bfs = self.bfs_reachable_first_food(&followed);
            self.choose_nearest_food(&followed, &next_bfs)
                .map(|target| target.dist)
        } else {
            Some(0)
        };
        let reexpand_bfs = self.bfs_reachable_first_food(&followed);

        SafeCollectBiteEval {
            followup_food_collected: zigzag_followup.is_some(),
            projected_prefix_len: self.project_prefix_after_resume(&followed),
            prefix_len: self.prefix_len(&followed),
            next_target_dist,
            next_fallback_dist,
            reachable_cell_count: reexpand_bfs.reachable_cell_count(),
            reachable_food_count: reexpand_bfs.reachable_food_count(&followed),
            bitten_length: bitten.colors.len(),
            followup_len: zigzag_followup.as_ref().map_or(usize::MAX, |moves| moves.len()),
        }
    }

    fn is_better_safe_collect_bite_candidate(
        &self,
        candidate_eval: SafeCollectBiteEval,
        candidate_moves_len: usize,
        current_eval: SafeCollectBiteEval,
        current_moves_len: usize,
    ) -> bool {
        candidate_eval.followup_food_collected && !current_eval.followup_food_collected
            || (candidate_eval.followup_food_collected == current_eval.followup_food_collected
                && candidate_eval.projected_prefix_len > current_eval.projected_prefix_len)
            || (candidate_eval.followup_food_collected == current_eval.followup_food_collected
                && candidate_eval.projected_prefix_len == current_eval.projected_prefix_len
                && candidate_eval.prefix_len > current_eval.prefix_len)
            || (candidate_eval.followup_food_collected == current_eval.followup_food_collected
                && candidate_eval.projected_prefix_len == current_eval.projected_prefix_len
                && candidate_eval.prefix_len == current_eval.prefix_len
                && candidate_eval.next_target_dist.is_some()
                && current_eval.next_target_dist.is_none())
            || (candidate_eval.followup_food_collected == current_eval.followup_food_collected
                && candidate_eval.projected_prefix_len == current_eval.projected_prefix_len
                && candidate_eval.prefix_len == current_eval.prefix_len
                && candidate_eval.next_target_dist.is_some() == current_eval.next_target_dist.is_some()
                && candidate_eval.next_target_dist.unwrap_or(usize::MAX)
                    < current_eval.next_target_dist.unwrap_or(usize::MAX))
            || (candidate_eval.followup_food_collected == current_eval.followup_food_collected
                && candidate_eval.projected_prefix_len == current_eval.projected_prefix_len
                && candidate_eval.prefix_len == current_eval.prefix_len
                && candidate_eval.next_target_dist == current_eval.next_target_dist
                && candidate_eval.next_fallback_dist.is_some()
                && current_eval.next_fallback_dist.is_none())
            || (candidate_eval.followup_food_collected == current_eval.followup_food_collected
                && candidate_eval.projected_prefix_len == current_eval.projected_prefix_len
                && candidate_eval.prefix_len == current_eval.prefix_len
                && candidate_eval.next_target_dist == current_eval.next_target_dist
                && candidate_eval.next_fallback_dist.is_some()
                    == current_eval.next_fallback_dist.is_some()
                && candidate_eval.next_fallback_dist.unwrap_or(usize::MAX)
                    < current_eval.next_fallback_dist.unwrap_or(usize::MAX))
            || (candidate_eval.followup_food_collected == current_eval.followup_food_collected
                && candidate_eval.projected_prefix_len == current_eval.projected_prefix_len
                && candidate_eval.prefix_len == current_eval.prefix_len
                && candidate_eval.next_target_dist == current_eval.next_target_dist
                && candidate_eval.next_fallback_dist == current_eval.next_fallback_dist
                && candidate_eval.reachable_food_count > current_eval.reachable_food_count)
            || (candidate_eval.followup_food_collected == current_eval.followup_food_collected
                && candidate_eval.projected_prefix_len == current_eval.projected_prefix_len
                && candidate_eval.prefix_len == current_eval.prefix_len
                && candidate_eval.next_target_dist == current_eval.next_target_dist
                && candidate_eval.next_fallback_dist == current_eval.next_fallback_dist
                && candidate_eval.reachable_food_count == current_eval.reachable_food_count
                && candidate_eval.reachable_cell_count > current_eval.reachable_cell_count)
            || (candidate_eval.followup_food_collected == current_eval.followup_food_collected
                && candidate_eval.projected_prefix_len == current_eval.projected_prefix_len
                && candidate_eval.prefix_len == current_eval.prefix_len
                && candidate_eval.next_target_dist == current_eval.next_target_dist
                && candidate_eval.next_fallback_dist == current_eval.next_fallback_dist
                && candidate_eval.reachable_food_count == current_eval.reachable_food_count
                && candidate_eval.reachable_cell_count == current_eval.reachable_cell_count
                && candidate_eval.bitten_length > current_eval.bitten_length)
            || (candidate_eval.followup_food_collected == current_eval.followup_food_collected
                && candidate_eval.projected_prefix_len == current_eval.projected_prefix_len
                && candidate_eval.prefix_len == current_eval.prefix_len
                && candidate_eval.next_target_dist == current_eval.next_target_dist
                && candidate_eval.next_fallback_dist == current_eval.next_fallback_dist
                && candidate_eval.reachable_food_count == current_eval.reachable_food_count
                && candidate_eval.reachable_cell_count == current_eval.reachable_cell_count
                && candidate_eval.bitten_length == current_eval.bitten_length
                && candidate_eval.followup_len < current_eval.followup_len)
            || (candidate_eval == current_eval && candidate_moves_len < current_moves_len)
    }

    fn project_prefix_after_resume(&self, state: &SnakeState) -> usize {
        let mut resumed = SnakeState {
            board: state.board.clone(),
            positions: state.positions.clone(),
            colors: state.colors.clone(),
        };

        let plan = self
            .plan_greedy_target_inner(&resumed).map(|(plan, _, _, _)| plan)
            .or_else(|| self.plan_greedy_fallback_inner(&resumed).map(|(plan, _, _)| plan))
            .or_else(|| self.plan_zigzag_safe_collect_moves(&resumed).map(|moves| Plan {
                phase: Phase::SafeCollect,
                moves,
                resume_greedy_after_apply: false,
            }));

        if let Some(plan) = plan {
            self.apply_moves(&mut resumed, &plan.moves);
        }

        self.prefix_len(&resumed)
    }

    fn should_force_safe_collect(&self) -> bool {
        self.start.elapsed().as_millis() >= SAFE_COLLECT_TIME_LIMIT_MS
            || self.ops.len() >= SAFE_COLLECT_TURN_LIMIT
    }

    fn plan_greedy_target_inner(
        &self,
        state: &SnakeState,
    ) -> Option<(Plan, BfsResult, FoodTarget, usize)> {
        let target_color = self.input.d[state.colors.len()];
        let primary_bfs =
            self.bfs_reachable_target_color_with_order(state, target_color, &TARGET_BFS_ORDERS[0]);
        let targets =
            self.choose_top_foods_of_color(state, &primary_bfs, target_color, GREEDY_TARGET_CANDIDATE_LIMIT);
        let mut best: Option<(GreedyTargetEval, usize, Plan, BfsResult, FoodTarget)> = None;

        for target in targets {
            let mut seen_moves = Vec::new();
            for order in TARGET_BFS_ORDERS {
                let bfs = self.bfs_reachable_target_color_with_order(state, target_color, &order);
                let Some(moves) = bfs.restore_moves(target.cell) else {
                    continue;
                };
                if seen_moves.iter().any(|existing: &Vec<char>| *existing == moves) {
                    continue;
                }
                seen_moves.push(moves.clone());

                let eval = self.evaluate_greedy_target_candidate(state, &moves);
                let candidate = (
                    eval,
                    moves.len(),
                    Plan {
                        phase: Phase::GreedyTarget,
                        moves,
                        resume_greedy_after_apply: false,
                    },
                    bfs,
                    target,
                );

                if best.as_ref().is_none_or(|current| {
                    self.is_better_greedy_target_candidate(
                        candidate.0,
                        candidate.1,
                        current.0,
                        current.1,
                    )
                }) {
                    best = Some(candidate);
                }
            }
        }

        let (_, _, plan, bfs, target) = best?;
        Some((plan, bfs, target, target_color))
    }

    fn is_better_greedy_target_candidate(
        &self,
        candidate_eval: GreedyTargetEval,
        candidate_moves_len: usize,
        current_eval: GreedyTargetEval,
        current_moves_len: usize,
    ) -> bool {
        candidate_eval.consecutive_target_hits_3 > current_eval.consecutive_target_hits_3
            || (candidate_eval.consecutive_target_hits_3 == current_eval.consecutive_target_hits_3
                && candidate_eval.consecutive_target_hits > current_eval.consecutive_target_hits)
            || (candidate_eval.consecutive_target_hits_3 == current_eval.consecutive_target_hits_3
                && candidate_eval.consecutive_target_hits == current_eval.consecutive_target_hits
                && candidate_eval.next_target_dist.is_some()
                && current_eval.next_target_dist.is_none())
            || (candidate_eval.consecutive_target_hits_3 == current_eval.consecutive_target_hits_3
                && candidate_eval.consecutive_target_hits == current_eval.consecutive_target_hits
                && candidate_eval.next_target_dist.is_some() == current_eval.next_target_dist.is_some()
                && candidate_eval.next_target_dist.unwrap_or(usize::MAX)
                    < current_eval.next_target_dist.unwrap_or(usize::MAX))
            || (candidate_eval.consecutive_target_hits_3 == current_eval.consecutive_target_hits_3
                && candidate_eval.consecutive_target_hits == current_eval.consecutive_target_hits
                && candidate_eval.next_target_dist == current_eval.next_target_dist
                && candidate_eval.next_fallback_dist.is_some()
                && current_eval.next_fallback_dist.is_none())
            || (candidate_eval.consecutive_target_hits_3 == current_eval.consecutive_target_hits_3
                && candidate_eval.consecutive_target_hits == current_eval.consecutive_target_hits
                && candidate_eval.next_target_dist == current_eval.next_target_dist
                && candidate_eval.next_fallback_dist.is_some()
                    == current_eval.next_fallback_dist.is_some()
                && candidate_eval.next_fallback_dist.unwrap_or(usize::MAX)
                    < current_eval.next_fallback_dist.unwrap_or(usize::MAX))
            || (candidate_eval.consecutive_target_hits_3 == current_eval.consecutive_target_hits_3
                && candidate_eval.consecutive_target_hits == current_eval.consecutive_target_hits
                && candidate_eval.next_target_dist == current_eval.next_target_dist
                && candidate_eval.next_fallback_dist == current_eval.next_fallback_dist
                && candidate_eval.reachable_food_count > current_eval.reachable_food_count)
            || (candidate_eval.consecutive_target_hits_3 == current_eval.consecutive_target_hits_3
                && candidate_eval.consecutive_target_hits == current_eval.consecutive_target_hits
                && candidate_eval.next_target_dist == current_eval.next_target_dist
                && candidate_eval.next_fallback_dist == current_eval.next_fallback_dist
                && candidate_eval.reachable_food_count == current_eval.reachable_food_count
                && candidate_eval.reachable_cell_count > current_eval.reachable_cell_count)
            || (candidate_eval.consecutive_target_hits_3 == current_eval.consecutive_target_hits_3
                && candidate_eval.consecutive_target_hits == current_eval.consecutive_target_hits
                && candidate_eval.next_target_dist == current_eval.next_target_dist
                && candidate_eval.next_fallback_dist == current_eval.next_fallback_dist
                && candidate_eval.reachable_food_count == current_eval.reachable_food_count
                && candidate_eval.reachable_cell_count == current_eval.reachable_cell_count
                && candidate_eval.prefix_len > current_eval.prefix_len)
            || (candidate_eval.consecutive_target_hits_3 == current_eval.consecutive_target_hits_3
                && candidate_eval.consecutive_target_hits == current_eval.consecutive_target_hits
                && candidate_eval.next_target_dist == current_eval.next_target_dist
                && candidate_eval.next_fallback_dist == current_eval.next_fallback_dist
                && candidate_eval.reachable_food_count == current_eval.reachable_food_count
                && candidate_eval.reachable_cell_count == current_eval.reachable_cell_count
                && candidate_eval.prefix_len == current_eval.prefix_len
                && candidate_moves_len < current_moves_len)
    }

    fn count_consecutive_target_hits(&self, state: &SnakeState, limit: usize) -> usize {
        let mut simulated = SnakeState {
            board: state.board.clone(),
            positions: state.positions.clone(),
            colors: state.colors.clone(),
        };
        let mut count = 0;

        while count < limit {
            if simulated.colors.len() >= self.input.m {
                return limit;
            }
            let target_color = self.input.d[simulated.colors.len()];
            let bfs = self.bfs_reachable_target_color(&simulated, target_color);
            let Some(target) = self.choose_nearest_food_of_color(&simulated, &bfs, target_color) else {
                break;
            };
            let Some(moves) = bfs.restore_moves(target.cell) else {
                break;
            };
            self.apply_moves(&mut simulated, &moves);
            count += 1;
        }

        count
    }

    fn evaluate_greedy_target_candidate(&self, state: &SnakeState, moves: &[char]) -> GreedyTargetEval {
        let mut next_state = SnakeState {
            board: state.board.clone(),
            positions: state.positions.clone(),
            colors: state.colors.clone(),
        };
        self.apply_moves(&mut next_state, moves);
        let consecutive_target_hits_3 = self.count_consecutive_target_hits(&next_state, 3);

        let next_target_dist = if next_state.colors.len() < self.input.m {
            let next_target_color = self.input.d[next_state.colors.len()];
            let next_bfs = self.bfs_reachable_target_color(&next_state, next_target_color);
            self.choose_nearest_food_of_color(&next_state, &next_bfs, next_target_color)
                .map(|target| target.dist)
        } else {
            Some(0)
        };
        let next_fallback_dist = if next_state.colors.len() < self.input.m {
            let next_bfs = self.bfs_reachable_first_food(&next_state);
            self.choose_nearest_food(&next_state, &next_bfs)
                .map(|target| target.dist)
        } else {
            Some(0)
        };
        let reexpand_bfs = self.bfs_reachable_first_food(&next_state);

        GreedyTargetEval {
            consecutive_target_hits_3,
            consecutive_target_hits: consecutive_target_hits_3.min(2),
            next_target_dist,
            next_fallback_dist,
            reachable_cell_count: reexpand_bfs.reachable_cell_count(),
            reachable_food_count: reexpand_bfs.reachable_food_count(&next_state),
            prefix_len: self.prefix_len(&next_state),
        }
    }

    fn plan_greedy_fallback_inner(
        &self,
        state: &SnakeState,
    ) -> Option<(Plan, BfsResult, FoodTarget)> {
        let primary_bfs = self.bfs_reachable_first_food_with_order(state, &TARGET_BFS_ORDERS[0]);
        let targets =
            self.choose_top_foods(state, &primary_bfs, GREEDY_FALLBACK_CANDIDATE_LIMIT);
        let mut best: Option<((bool, usize, bool, usize, usize), usize, Plan, BfsResult, FoodTarget)> =
            None;

        for target in targets {
            let mut seen_moves = Vec::new();
            for order in TARGET_BFS_ORDERS {
                let bfs = self.bfs_reachable_first_food_with_order(state, &order);
                let Some(moves) = bfs.restore_moves(target.cell) else {
                    continue;
                };
                if seen_moves.iter().any(|existing: &Vec<char>| *existing == moves) {
                    continue;
                }
                seen_moves.push(moves.clone());

                let eval = self.evaluate_greedy_fallback_candidate(state, &moves);
                let candidate = (
                    eval,
                    moves.len(),
                    Plan {
                        phase: Phase::GreedyFallback,
                        moves,
                        resume_greedy_after_apply: false,
                    },
                    bfs,
                    target,
                );

                if best.as_ref().is_none_or(|current| {
                    candidate.0 .0 && !current.0 .0
                        || (candidate.0 .0 == current.0 .0 && candidate.0 .1 < current.0 .1)
                        || (candidate.0 .0 == current.0 .0
                            && candidate.0 .1 == current.0 .1
                            && candidate.0 .2
                            && !current.0 .2)
                        || (candidate.0 .0 == current.0 .0
                            && candidate.0 .1 == current.0 .1
                            && candidate.0 .2 == current.0 .2
                            && candidate.0 .3 < current.0 .3)
                        || (candidate.0 .0 == current.0 .0
                            && candidate.0 .1 == current.0 .1
                            && candidate.0 .2 == current.0 .2
                            && candidate.0 .3 == current.0 .3
                            && candidate.0 .4 > current.0 .4)
                        || (candidate.0 == current.0 && candidate.1 < current.1)
                }) {
                    best = Some(candidate);
                }
            }
        }

        let (_, _, plan, bfs, target) = best?;
        Some((plan, bfs, target))
    }

    fn evaluate_greedy_fallback_candidate(
        &self,
        state: &SnakeState,
        moves: &[char],
    ) -> (bool, usize, bool, usize, usize) {
        let mut next_state = SnakeState {
            board: state.board.clone(),
            positions: state.positions.clone(),
            colors: state.colors.clone(),
        };
        self.apply_moves(&mut next_state, moves);

        let next_target_dist = if next_state.colors.len() < self.input.m {
            let next_target_color = self.input.d[next_state.colors.len()];
            let next_bfs = self.bfs_reachable_target_color(&next_state, next_target_color);
            self.choose_nearest_food_of_color(&next_state, &next_bfs, next_target_color)
                .map(|target| target.dist)
        } else {
            Some(0)
        };
        let next_fallback_dist = if next_state.colors.len() < self.input.m {
            let next_bfs = self.bfs_reachable_first_food(&next_state);
            self.choose_nearest_food(&next_state, &next_bfs)
                .map(|target| target.dist)
        } else {
            Some(0)
        };

        (
            next_target_dist.is_some(),
            next_target_dist.unwrap_or(usize::MAX),
            next_fallback_dist.is_some(),
            next_fallback_dist.unwrap_or(usize::MAX),
            self.prefix_len(&next_state),
        )
    }

    #[cfg(debug_assertions)]
    fn reset_phase_trace(&self) {
        let _ = std::fs::File::create("debug.txt");
    }

    #[cfg(not(debug_assertions))]
    fn reset_phase_trace(&self) {}

    #[cfg(debug_assertions)]
    fn reset_debug_answer(&self) {
        let _ = std::fs::File::create("debug_ans.txt");
    }

    #[cfg(not(debug_assertions))]
    fn reset_debug_answer(&self) {}

    #[cfg(debug_assertions)]
    fn write_phase_trace(&self, turn: usize, phase: Phase) {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("debug.txt")
        {
            let _ = writeln!(file, "{} {}", turn, phase.as_str());
        }
    }

    #[cfg(not(debug_assertions))]
    fn write_phase_trace(&self, _turn: usize, _phase: Phase) {}

    #[cfg(debug_assertions)]
    fn write_debug_answer(&self) {
        if let Ok(mut file) = std::fs::File::create("debug_ans.txt") {
            for &op in &self.ops {
                let _ = writeln!(file, "{op}");
            }
        }
    }

    #[cfg(not(debug_assertions))]
    fn write_debug_answer(&self) {}

    #[cfg(debug_assertions)]
    fn write_bfs_snapshot(
        &self,
        turn: usize,
        phase: Phase,
        target_color: Option<usize>,
        target: (usize, usize),
        bfs: &BfsResult,
    ) {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("debug.txt")
        {
            let _ = writeln!(file, "TURN {}", turn);
            let _ = writeln!(file, "PHASE {}", phase.as_str());
            if let Some(target_color) = target_color {
                let _ = writeln!(file, "TARGET_COLOR {}", target_color);
            }
            let _ = writeln!(file, "TARGET {} {}", target.0, target.1);
            let _ = writeln!(file, "BFS");
            let _ = write!(file, "{}", bfs.dist_debug_text());
            let _ = writeln!(file, "END");
        }
    }

    #[cfg(not(debug_assertions))]
    fn write_bfs_snapshot(
        &self,
        _turn: usize,
        _phase: Phase,
        _target_color: Option<usize>,
        _target: (usize, usize),
        _bfs: &BfsResult,
    ) {
    }

    fn should_launch_safe_branch(&self, state: &SnakeState, plan: &Plan) -> bool {
        !self.force_safe_collect_mode
            && state.colors.len() < self.input.m
            && plan.phase == Phase::SafeCollect
            && plan.resume_greedy_after_apply
            && (self.remaining_food_count(state) <= SAFE_BRANCH_ENDGAME_REMAINING_FOOD_LIMIT
                || self.is_progress_stalled()
                || self.safe_collect_count + 1 >= SAFE_BRANCH_MIN_SAFE_COLLECT_COUNT)
    }

    fn bfs_reachable_body_target(&self, state: &SnakeState, target: (usize, usize)) -> BfsResult {
        let start = state.positions[0];
        let mut result = BfsResult::new(self.input.n, start);
        let mut blocked = vec![vec![false; self.input.n]; self.input.n];
        let mut queue = VecDeque::new();

        for &(i, j) in state.positions.iter().skip(1) {
            blocked[i][j] = true;
        }
        for i in 0..self.input.n {
            for j in 0..self.input.n {
                if state.board[i][j] != 0 {
                    blocked[i][j] = true;
                }
            }
        }
        blocked[target.0][target.1] = false;

        result.reachable[start.0][start.1] = true;
        result.dist[start.0][start.1] = Some(0);
        queue.push_back(start);

        while let Some(current) = queue.pop_front() {
            let current_dist =
                result.dist[current.0][current.1].expect("visited cell must have distance");

            for op in ['U', 'D', 'L', 'R'] {
                let Some(next) = self.try_advance(current, op) else {
                    continue;
                };
                if blocked[next.0][next.1] || result.reachable[next.0][next.1] {
                    continue;
                }

                result.reachable[next.0][next.1] = true;
                result.dist[next.0][next.1] = Some(current_dist + 1);
                result.parent[next.0][next.1] = Some(current);
                result.parent_move[next.0][next.1] = Some(op);
                if next == target {
                    return result;
                }
                queue.push_back(next);
            }
        }

        result
    }

    fn plan_zigzag_safe_collect_moves(&self, state: &SnakeState) -> Option<Vec<char>> {
        let path = self.build_zigzag_path();
        let start = state.positions[0];
        let start_idx = path.iter().position(|&cell| cell == start)?;
        let mut blocked = vec![vec![false; self.input.n]; self.input.n];
        for &(i, j) in state.positions.iter().skip(1) {
            blocked[i][j] = true;
        }

        let forward = self.collect_along_zigzag_direction(state, &path, &blocked, start_idx, 1);
        let backward = self.collect_along_zigzag_direction(state, &path, &blocked, start_idx, -1);

        match (forward, backward) {
            (Some(left), Some(right)) => {
                let left_eval = self.evaluate_safe_collect_route_candidate(state, &left);
                let right_eval = self.evaluate_safe_collect_route_candidate(state, &right);
                if self.is_better_safe_collect_route_candidate(left_eval, left.len(), right_eval, right.len()) {
                    Some(left)
                } else {
                    Some(right)
                }
            }
            (Some(moves), None) | (None, Some(moves)) => Some(moves),
            (None, None) => None,
        }
    }

    fn collect_along_zigzag_direction(
        &self,
        state: &SnakeState,
        path: &[(usize, usize)],
        blocked: &[Vec<bool>],
        start_idx: usize,
        delta: isize,
    ) -> Option<Vec<char>> {
        let mut current = path[start_idx];
        let mut idx = start_idx as isize;
        let mut moves = Vec::new();

        loop {
            idx += delta;
            if !(0..path.len() as isize).contains(&idx) {
                return None;
            }

            let next = path[idx as usize];
            if blocked[next.0][next.1] {
                return None;
            }

            moves.push(Self::step(current, next));
            if state.board[next.0][next.1] != 0 {
                return Some(moves);
            }

            current = next;
        }
    }

    fn evaluate_safe_collect_route_candidate(
        &self,
        state: &SnakeState,
        moves: &[char],
    ) -> SafeCollectRouteEval {
        let mut next_state = SnakeState {
            board: state.board.clone(),
            positions: state.positions.clone(),
            colors: state.colors.clone(),
        };
        self.apply_moves(&mut next_state, moves);

        let next_target_dist = if next_state.colors.len() < self.input.m {
            let next_target_color = self.input.d[next_state.colors.len()];
            let next_bfs = self.bfs_reachable_target_color(&next_state, next_target_color);
            self.choose_nearest_food_of_color(&next_state, &next_bfs, next_target_color)
                .map(|target| target.dist)
        } else {
            Some(0)
        };
        let next_fallback_dist = if next_state.colors.len() < self.input.m {
            let next_bfs = self.bfs_reachable_first_food(&next_state);
            self.choose_nearest_food(&next_state, &next_bfs)
                .map(|target| target.dist)
        } else {
            Some(0)
        };
        let reexpand_bfs = self.bfs_reachable_first_food(&next_state);

        SafeCollectRouteEval {
            projected_prefix_len: self.project_prefix_after_resume(&next_state),
            prefix_len: self.prefix_len(&next_state),
            next_target_dist,
            next_fallback_dist,
            reachable_cell_count: reexpand_bfs.reachable_cell_count(),
            reachable_food_count: reexpand_bfs.reachable_food_count(&next_state),
        }
    }

    fn is_better_safe_collect_route_candidate(
        &self,
        candidate_eval: SafeCollectRouteEval,
        candidate_moves_len: usize,
        current_eval: SafeCollectRouteEval,
        current_moves_len: usize,
    ) -> bool {
        candidate_eval.projected_prefix_len > current_eval.projected_prefix_len
            || (candidate_eval.projected_prefix_len == current_eval.projected_prefix_len
                && candidate_eval.prefix_len > current_eval.prefix_len)
            || (candidate_eval.projected_prefix_len == current_eval.projected_prefix_len
                && candidate_eval.prefix_len == current_eval.prefix_len
                && candidate_eval.next_target_dist.is_some()
                && current_eval.next_target_dist.is_none())
            || (candidate_eval.projected_prefix_len == current_eval.projected_prefix_len
                && candidate_eval.prefix_len == current_eval.prefix_len
                && candidate_eval.next_target_dist.is_some() == current_eval.next_target_dist.is_some()
                && candidate_eval.next_target_dist.unwrap_or(usize::MAX)
                    < current_eval.next_target_dist.unwrap_or(usize::MAX))
            || (candidate_eval.projected_prefix_len == current_eval.projected_prefix_len
                && candidate_eval.prefix_len == current_eval.prefix_len
                && candidate_eval.next_target_dist == current_eval.next_target_dist
                && candidate_eval.next_fallback_dist.is_some()
                && current_eval.next_fallback_dist.is_none())
            || (candidate_eval.projected_prefix_len == current_eval.projected_prefix_len
                && candidate_eval.prefix_len == current_eval.prefix_len
                && candidate_eval.next_target_dist == current_eval.next_target_dist
                && candidate_eval.next_fallback_dist.is_some()
                    == current_eval.next_fallback_dist.is_some()
                && candidate_eval.next_fallback_dist.unwrap_or(usize::MAX)
                    < current_eval.next_fallback_dist.unwrap_or(usize::MAX))
            || (candidate_eval.projected_prefix_len == current_eval.projected_prefix_len
                && candidate_eval.prefix_len == current_eval.prefix_len
                && candidate_eval.next_target_dist == current_eval.next_target_dist
                && candidate_eval.next_fallback_dist == current_eval.next_fallback_dist
                && candidate_eval.reachable_food_count > current_eval.reachable_food_count)
            || (candidate_eval.projected_prefix_len == current_eval.projected_prefix_len
                && candidate_eval.prefix_len == current_eval.prefix_len
                && candidate_eval.next_target_dist == current_eval.next_target_dist
                && candidate_eval.next_fallback_dist == current_eval.next_fallback_dist
                && candidate_eval.reachable_food_count == current_eval.reachable_food_count
                && candidate_eval.reachable_cell_count > current_eval.reachable_cell_count)
            || (candidate_eval == current_eval && candidate_moves_len < current_moves_len)
    }

    fn update_progress(
        &mut self,
        state: &SnakeState,
        phase: Phase,
        resume_greedy_after_apply: bool,
    ) {
        if self.force_safe_collect_mode {
            self.safe_collect_active = state.colors.len() < self.input.m;
        } else if phase == Phase::BiteRebuild {
            self.safe_collect_active = false;
        } else if resume_greedy_after_apply {
            self.safe_collect_active = false;
        }
        let previous = self.last_progress;
        let prefix_len = self.prefix_len(state);
        let remaining_food_count = self.remaining_food_count(state);
        let can_reach_any_food = self.can_reach_any_food(state);
        let fallback_stall_count =
            self.fallback_stall_count(previous, prefix_len, remaining_food_count, phase);

        self.last_progress = Some(ProgressSnapshot {
            prefix_len,
            remaining_food_count,
            can_reach_any_food,
            fallback_stall_count,
            is_stalled: fallback_stall_count >= FALLBACK_STALL_LIMIT,
        });
    }

    fn prefix_len(&self, state: &SnakeState) -> usize {
        state.colors
            .iter()
            .zip(self.input.d.iter())
            .take_while(|(actual, desired)| actual == desired)
            .count()
    }

    fn remaining_food_count(&self, state: &SnakeState) -> usize {
        state.board
            .iter()
            .map(|row| row.iter().filter(|&&food| food != 0).count())
            .sum()
    }

    fn can_reach_any_food(&self, state: &SnakeState) -> bool {
        let bfs = self.bfs_reachable_first_food(state);
        self.choose_nearest_food(state, &bfs).is_some()
    }

    fn fallback_stall_count(
        &self,
        previous: Option<ProgressSnapshot>,
        prefix_len: usize,
        remaining_food_count: usize,
        phase: Phase,
    ) -> usize {
        let Some(previous) = previous else {
            return 0;
        };
        if phase != Phase::GreedyFallback {
            return 0;
        }

        let prefix_grew = prefix_len > previous.prefix_len;
        let food_collected = remaining_food_count < previous.remaining_food_count;
        if food_collected && !prefix_grew {
            previous.fallback_stall_count + 1
        } else {
            0
        }
    }

    fn is_progress_stalled(&self) -> bool {
        self.last_progress
            .is_some_and(|progress| progress.is_stalled)
    }

    fn bfs_reachable_cells(&self, state: &SnakeState) -> BfsResult {
        let start = state.positions[0];
        let mut result = BfsResult::new(self.input.n, start);
        let mut blocked = vec![vec![false; self.input.n]; self.input.n];
        let mut queue = VecDeque::new();

        // 噛み切りなしの初期版では、現在の頭以外の蛇マスを障害物とみなす。
        for &(i, j) in state.positions.iter().skip(1) {
            blocked[i][j] = true;
        }

        result.reachable[start.0][start.1] = true;
        result.dist[start.0][start.1] = Some(0);
        queue.push_back(start);

        while let Some(current) = queue.pop_front() {
            let current_dist =
                result.dist[current.0][current.1].expect("visited cell must have distance");

            for op in ['U', 'D', 'L', 'R'] {
                let Some(next) = self.try_advance(current, op) else {
                    continue;
                };
                if blocked[next.0][next.1] || result.reachable[next.0][next.1] {
                    continue;
                }

                result.reachable[next.0][next.1] = true;
                result.dist[next.0][next.1] = Some(current_dist + 1);
                result.parent[next.0][next.1] = Some(current);
                result.parent_move[next.0][next.1] = Some(op);
                queue.push_back(next);
            }
        }

        result
    }

    fn bfs_reachable_first_food(&self, state: &SnakeState) -> BfsResult {
        self.bfs_reachable_first_food_with_order(state, &TARGET_BFS_ORDERS[0])
    }

    fn bfs_reachable_first_food_with_order(
        &self,
        state: &SnakeState,
        order: &[char; 4],
    ) -> BfsResult {
        let start = state.positions[0];
        let mut result = BfsResult::new(self.input.n, start);
        let cell_open_turn = self.build_cell_open_turn(state);
        let mut queue = VecDeque::new();

        result.reachable[start.0][start.1] = true;
        result.dist[start.0][start.1] = Some(0);
        queue.push_back(start);

        while let Some(current) = queue.pop_front() {
            let current_dist =
                result.dist[current.0][current.1].expect("visited cell must have distance");

            // 任意餌 fallback でも、最初に到達した餌で一度止まって再計画する。
            if current != start && state.board[current.0][current.1] != 0 {
                continue;
            }

            for &op in order {
                let Some(next) = self.try_advance(current, op) else {
                    continue;
                };
                let arrival_turn = current_dist + 1;
                let still_occupied = cell_open_turn[next.0][next.1] > arrival_turn;
                if still_occupied || result.reachable[next.0][next.1] {
                    continue;
                }

                result.reachable[next.0][next.1] = true;
                result.dist[next.0][next.1] = Some(arrival_turn);
                result.parent[next.0][next.1] = Some(current);
                result.parent_move[next.0][next.1] = Some(op);
                queue.push_back(next);
            }
        }

        result
    }

    fn bfs_reachable_target_color(&self, state: &SnakeState, target_color: usize) -> BfsResult {
        self.bfs_reachable_target_color_with_order(state, target_color, &TARGET_BFS_ORDERS[0])
    }

    fn bfs_reachable_target_color_with_order(
        &self,
        state: &SnakeState,
        target_color: usize,
        order: &[char; 4],
    ) -> BfsResult {
        let start = state.positions[0];
        let mut result = BfsResult::new(self.input.n, start);
        let mut cell_open_turn = self.build_cell_open_turn(state);
        let mut queue = VecDeque::new();
        for i in 0..self.input.n {
            for j in 0..self.input.n {
                let food = state.board[i][j];
                if food != 0 && food != target_color {
                    cell_open_turn[i][j] = usize::MAX;
                }
            }
        }

        result.reachable[start.0][start.1] = true;
        result.dist[start.0][start.1] = Some(0);
        queue.push_back(start);

        while let Some(current) = queue.pop_front() {
            let current_dist =
                result.dist[current.0][current.1].expect("visited cell must have distance");

            // 同色餌に到達したら、そこで止まる経路だけを候補にしたいので先へは展開しない。
            if current != start && state.board[current.0][current.1] == target_color {
                continue;
            }

            for &op in order {
                let Some(next) = self.try_advance(current, op) else {
                    continue;
                };
                let arrival_turn = current_dist + 1;
                let still_occupied = cell_open_turn[next.0][next.1] > arrival_turn;
                if still_occupied || result.reachable[next.0][next.1] {
                    continue;
                }

                result.reachable[next.0][next.1] = true;
                result.dist[next.0][next.1] = Some(arrival_turn);
                result.parent[next.0][next.1] = Some(current);
                result.parent_move[next.0][next.1] = Some(op);
                queue.push_back(next);
            }
        }

        result
    }

    fn build_cell_open_turn(&self, state: &SnakeState) -> Vec<Vec<usize>> {
        let mut cell_open_turn = vec![vec![0; self.input.n]; self.input.n];
        for (l, &(i, j)) in state.positions.iter().enumerate().skip(1).rev() {
            cell_open_turn[i][j] = state.colors.len() - l - 1;
        }
        cell_open_turn
    }

    fn choose_nearest_food_of_color(
        &self,
        state: &SnakeState,
        bfs: &BfsResult,
        target_color: usize,
    ) -> Option<FoodTarget> {
        let mut best = None;

        for i in 0..self.input.n {
            for j in 0..self.input.n {
                if state.board[i][j] != target_color {
                    continue;
                }
                let Some(dist) = bfs.distance((i, j)) else {
                    continue;
                };
                let candidate = FoodTarget { cell: (i, j), dist };
                if best.is_none_or(|current: FoodTarget| {
                    candidate.dist < current.dist
                        || (candidate.dist == current.dist && candidate.cell < current.cell)
                }) {
                    best = Some(candidate);
                }
            }
        }

        best
    }

    fn choose_top_foods(&self, state: &SnakeState, bfs: &BfsResult, limit: usize) -> Vec<FoodTarget> {
        let mut candidates = Vec::new();

        for i in 0..self.input.n {
            for j in 0..self.input.n {
                if state.board[i][j] == 0 {
                    continue;
                }
                let Some(dist) = bfs.distance((i, j)) else {
                    continue;
                };
                candidates.push(FoodTarget { cell: (i, j), dist });
            }
        }

        candidates.sort_by_key(|target| (target.dist, target.cell));
        candidates.truncate(limit);
        candidates
    }

    fn choose_top_foods_of_color(
        &self,
        state: &SnakeState,
        bfs: &BfsResult,
        target_color: usize,
        limit: usize,
    ) -> Vec<FoodTarget> {
        let mut candidates = Vec::new();

        for i in 0..self.input.n {
            for j in 0..self.input.n {
                if state.board[i][j] != target_color {
                    continue;
                }
                let Some(dist) = bfs.distance((i, j)) else {
                    continue;
                };
                candidates.push(FoodTarget { cell: (i, j), dist });
            }
        }

        candidates.sort_by_key(|target| (target.dist, target.cell));
        candidates.truncate(limit);
        candidates
    }

    fn choose_nearest_food(&self, state: &SnakeState, bfs: &BfsResult) -> Option<FoodTarget> {
        let mut best = None;

        for i in 0..self.input.n {
            for j in 0..self.input.n {
                if state.board[i][j] == 0 {
                    continue;
                }
                let Some(dist) = bfs.distance((i, j)) else {
                    continue;
                };
                let candidate = FoodTarget { cell: (i, j), dist };
                if best.is_none_or(|current: FoodTarget| {
                    candidate.dist < current.dist
                        || (candidate.dist == current.dist && candidate.cell < current.cell)
                }) {
                    best = Some(candidate);
                }
            }
        }

        best
    }

    fn apply_move(&self, state: &mut SnakeState, op: char) {
        let next_head = self.advance(state.positions[0], op);
        let previous_tail = *state.positions.last().expect("snake must be non-empty");

        state.positions.insert(0, next_head);
        state.positions.pop();

        let food = state.board[next_head.0][next_head.1];
        if food != 0 {
            state.board[next_head.0][next_head.1] = 0;
            state.positions.push(previous_tail);
            state.colors.push(food);
            return;
        }

        if let Some(h) = (1..state.positions.len().saturating_sub(1))
            .find(|&idx| state.positions[idx] == next_head)
        {
            for idx in (h + 1)..state.positions.len() {
                let (i, j) = state.positions[idx];
                state.board[i][j] = state.colors[idx];
            }
            state.positions.truncate(h + 1);
            state.colors.truncate(h + 1);
        }
    }

    fn advance(&self, from: (usize, usize), op: char) -> (usize, usize) {
        self.try_advance(from, op).expect("move must stay on board")
    }

    fn try_advance(&self, from: (usize, usize), op: char) -> Option<(usize, usize)> {
        let (di, dj) = match op {
            'U' => (-1, 0),
            'D' => (1, 0),
            'L' => (0, -1),
            'R' => (0, 1),
            _ => panic!("unknown operation"),
        };
        let ni = from.0.checked_add_signed(di)?;
        let nj = from.1.checked_add_signed(dj)?;
        if ni >= self.input.n || nj >= self.input.n {
            return None;
        }
        Some((ni, nj))
    }
}

impl BfsResult {
    fn new(n: usize, start: (usize, usize)) -> Self {
        Self {
            start,
            reachable: vec![vec![false; n]; n],
            dist: vec![vec![None; n]; n],
            parent: vec![vec![None; n]; n],
            parent_move: vec![vec![None; n]; n],
        }
    }

    fn can_reach(&self, cell: (usize, usize)) -> bool {
        self.reachable[cell.0][cell.1]
    }

    fn distance(&self, cell: (usize, usize)) -> Option<usize> {
        self.dist[cell.0][cell.1]
    }

    fn parent_of(&self, cell: (usize, usize)) -> Option<(usize, usize)> {
        self.parent[cell.0][cell.1]
    }

    fn restore_moves(&self, goal: (usize, usize)) -> Option<Vec<char>> {
        if !self.can_reach(goal) {
            return None;
        }

        let mut moves = Vec::new();
        let mut current = goal;
        while current != self.start {
            moves.push(self.parent_move[current.0][current.1]?);
            current = self.parent[current.0][current.1]?;
        }
        moves.reverse();
        Some(moves)
    }

    fn reachable_cell_count(&self) -> usize {
        self.reachable
            .iter()
            .map(|row| row.iter().filter(|&&cell| cell).count())
            .sum()
    }

    fn reachable_food_count(&self, state: &SnakeState) -> usize {
        let mut count = 0;
        for i in 0..self.reachable.len() {
            for j in 0..self.reachable[i].len() {
                if self.reachable[i][j] && state.board[i][j] != 0 {
                    count += 1;
                }
            }
        }
        count
    }

    fn dist_debug_text(&self) -> String {
        let mut lines = Vec::with_capacity(self.dist.len());
        for row in &self.dist {
            let line = row
                .iter()
                .map(|cell| match cell {
                    Some(dist) => dist.to_string(),
                    None => ".".to_owned(),
                })
                .collect::<Vec<_>>()
                .join(" ");
            lines.push(line);
        }
        let mut text = lines.join("\n");
        text.push('\n');
        text
    }
}

fn main() {
    let start = Instant::now();
    let input = read_input();
    let mut solver = Solver::new(0, start, input);

    solver.solve();
    solver.ans();
    solver.result();
}
