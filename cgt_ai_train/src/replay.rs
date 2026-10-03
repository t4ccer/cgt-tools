use crate::self_play::Examples;
use anyhow::{Context, Result, bail, ensure};
use burn::tensor::f16;
use cgt_ai_core::ruleset::Ruleset;
use std::{
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
};

const MAGIC: &[u8; 8] = b"CGTAIRB1";

pub struct ReplayBuffer<R: Ruleset> {
    rules: R,
    capacity: usize,
    num_actions: usize,
    states: Vec<R::State>,
    policies: Vec<f16>,
    outcomes: Vec<f32>,
    pos: usize,
}

impl<R: Ruleset> ReplayBuffer<R> {
    pub fn new(rules: R, capacity: usize) -> ReplayBuffer<R> {
        let num_actions = rules.num_actions();
        ReplayBuffer {
            rules,
            capacity,
            num_actions,
            states: Vec::with_capacity(capacity),
            policies: vec![f16::ZERO; capacity * num_actions],
            outcomes: vec![0.0; capacity],
            pos: 0,
        }
    }

    pub const fn len(&self) -> usize {
        self.states.len()
    }

    pub fn add(&mut self, examples: &Examples<R::State>) {
        let skip = examples.len().saturating_sub(self.capacity);
        for i in skip..examples.len() {
            let slot = self.pos;
            if slot == self.states.len() {
                self.states.push(examples.states[i]);
            } else {
                self.states[slot] = examples.states[i];
            }
            self.policies[slot * self.num_actions..(slot + 1) * self.num_actions]
                .copy_from_slice(examples.policy(i));
            self.outcomes[slot] = examples.outcomes[i];
            self.pos = (self.pos + 1) % self.capacity;
        }
    }

    pub fn state(&self, i: usize) -> &R::State {
        &self.states[i]
    }

    pub fn policy(&self, i: usize) -> &[f16] {
        &self.policies[i * self.num_actions..(i + 1) * self.num_actions]
    }

    pub fn outcome(&self, i: usize) -> f32 {
        self.outcomes[i]
    }

    fn header(&self, size: usize) -> Vec<u8> {
        let mut header = MAGIC.to_vec();
        let name = self.rules.name().as_bytes();
        for field in [
            name.len(),
            self.rules.state_bytes(),
            self.num_actions,
            size,
            self.pos,
        ] {
            header.extend_from_slice(&(field as u64).to_le_bytes());
        }
        header.extend_from_slice(name);
        header
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let tmp = path.with_extension("tmp");
        let mut out = BufWriter::new(std::fs::File::create(&tmp)?);
        out.write_all(&self.header(self.len()))?;
        let mut bytes = Vec::with_capacity(self.rules.state_bytes());
        for state in &self.states {
            bytes.clear();
            self.rules.write_state(state, &mut bytes);
            out.write_all(&bytes)?;
        }
        for p in &self.policies[..self.len() * self.num_actions] {
            out.write_all(&p.to_bits().to_le_bytes())?;
        }
        for z in &self.outcomes[..self.len()] {
            out.write_all(&z.to_le_bytes())?;
        }
        out.into_inner()?.sync_all()?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    pub fn load(&mut self, path: &Path) -> Result<()> {
        let mut input = BufReader::new(std::fs::File::open(path)?);
        let mut magic = [0; 8];
        input.read_exact(&mut magic)?;
        ensure!(&magic == MAGIC, "{} is not a replay buffer", path.display());
        let mut read_u64 = || -> Result<usize> {
            let mut word = [0; 8];
            input.read_exact(&mut word)?;
            Ok(u64::from_le_bytes(word) as usize)
        };
        let (name_len, state_bytes, num_actions, stored, pos) = (
            read_u64()?,
            read_u64()?,
            read_u64()?,
            read_u64()?,
            read_u64()?,
        );
        let mut name = vec![0; name_len];
        input.read_exact(&mut name)?;
        if name != self.rules.name().as_bytes() {
            bail!(
                "{} holds {} positions, not {}",
                path.display(),
                String::from_utf8_lossy(&name),
                self.rules.name()
            );
        }
        ensure!(
            state_bytes == self.rules.state_bytes() && num_actions == self.num_actions,
            "{} was written for another version of {}",
            path.display(),
            self.rules.name()
        );

        let mut states = vec![0; stored * state_bytes];
        input.read_exact(&mut states)?;
        let mut policies = vec![0; stored * num_actions * 2];
        input.read_exact(&mut policies)?;
        let mut outcomes = vec![0; stored * 4];
        input.read_exact(&mut outcomes)?;

        let n = stored.min(self.capacity);
        self.states = states
            .chunks_exact(state_bytes)
            .take(n)
            .map(|bytes| self.rules.read_state(bytes))
            .collect::<Option<_>>()
            .with_context(|| format!("{} holds invalid positions", path.display()))?;
        for (z, bytes) in self
            .outcomes
            .iter_mut()
            .zip(outcomes.as_chunks::<4>().0.iter().take(n))
        {
            *z = f32::from_le_bytes(*bytes);
        }
        for (p, bytes) in self.policies[..n * num_actions]
            .iter_mut()
            .zip(policies.as_chunks::<2>().0)
        {
            *p = f16::from_bits(u16::from_le_bytes(*bytes));
        }
        self.pos = if n == self.capacity {
            pos % self.capacity
        } else {
            n
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgt_ai_core::{quelhas::Quelhas, ruleset::random_position};
    use rand::{SeedableRng, rngs::SmallRng};

    #[test]
    fn save_load_roundtrip() {
        let mut rng = SmallRng::seed_from_u64(0);
        let num_actions = Quelhas.num_actions();
        let mut examples = Examples::new(num_actions);
        for i in 0..5 {
            examples.states.push(random_position(&Quelhas, i, &mut rng));
            let mut policy = vec![f16::ZERO; num_actions];
            policy[i] = f16::ONE;
            examples.policies.extend(policy);
            examples.outcomes.push(if i % 2 == 0 { 1.0 } else { -1.0 });
        }
        let mut buffer = ReplayBuffer::new(Quelhas, 3);
        buffer.add(&examples);
        assert_eq!(buffer.len(), 3);

        let path = std::env::temp_dir().join(format!("cgt-ai-replay-{}.bin", std::process::id()));
        buffer.save(&path).unwrap();
        let mut loaded = ReplayBuffer::new(Quelhas, 3);
        loaded.load(&path).unwrap();
        std::fs::remove_file(&path).unwrap();

        for i in 0..3 {
            assert_eq!(loaded.state(i), buffer.state(i));
            assert_eq!(loaded.policy(i), buffer.policy(i));
            assert_eq!(loaded.outcome(i).to_bits(), buffer.outcome(i).to_bits());
        }
        assert_eq!(loaded.pos, buffer.pos);
    }
}
