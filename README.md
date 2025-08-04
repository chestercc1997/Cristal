# Cristal

## Prerequisites

- Rust environment
- Berkeley ABC tool
- bc
- parallel 
  
(can be installed by running `bash install_parallel.sh`) and `bc` (`sudo apt-get install bc`)
## Build

```bash
make
```


## Usage


```bash
bash run_total_choice_mix.sh --case *.aig
e.g.
bash /data/cchen/choice/run_total_choice_mix.sh --case log2.aig
```
If you want to add new cases, please process your new AIG file with benchmarks/process.py and place the result in the exp_aig directory.Before running, make sure to execute:
benchmarks/exp/benchmarks/areaopt.sh
benchmarks/exp/benchmarks/delayopt.sh

This is required because Cristal's Rust parser needs the input and output signal names to be normalized. 
It ensures the ABC AIG extension parser can reliably add choices for further steps.
