import os
import sys

from matplotlib import pyplot as plt
from matplotlib.ticker import PercentFormatter

#
sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))
import nanoset

S = sys.getsizeof(set())
s_pico = sys.getsizeof(nanoset.PicoSet())
s_nano = sys.getsizeof(nanoset.NanoSet())

def with_set(x):
    return 100 * S / (100 * S)

def with_nanoset(x):
    return (x*(S + s_nano) + (100 - x)*s_nano) / (100 * S)

def with_picoset(x):
    return (x*(S + s_pico) + (100 - x)*s_pico) / (100 * S)

listx = list(range(101))
maxmem_nanoset = with_nanoset(100)
maxmem_picoset = with_picoset(100)
fill100_nanoset = min(range(101), key=lambda x: abs(1 - with_nanoset(x)))
fill100_picoset = min(range(101), key=lambda x: abs(1 - with_picoset(x)))

plt.figure(figsize=[12, 8])
plt.xkcd(scale=0.5)
plt.axvline(fill100_nanoset, linewidth=0.5, linestyle='--', color="grey")
plt.axvline(fill100_picoset, linewidth=0.5, linestyle='--', color="grey")
plt.axhline(maxmem_nanoset, linewidth=0.5, linestyle='--', color="grey")
plt.axhline(maxmem_picoset, linewidth=0.5, linestyle='--', color="grey")
plt.plot(listx, list(map(with_set, listx)), label="with $set$")
plt.plot(listx, list(map(with_nanoset, listx)), label="with $nanoset.NanoSet$")
plt.plot(listx, list(map(with_picoset, listx)), label="with $nanoset.PicoSet$")
plt.legend()
plt.xlim([0, 100])
plt.ylim([0, 1.4])
plt.gca().yaxis.set_major_formatter(PercentFormatter(xmax=1))
plt.gca().xaxis.set_major_formatter(PercentFormatter(xmax=100))
plt.xlabel("Ratio of non-empty sets")
plt.ylabel("Ratio of memory used")

plt.annotate(f'{maxmem_nanoset:.0%}', xy=(0, maxmem_nanoset), xytext=(101, maxmem_nanoset), color="grey")
plt.annotate(f'{maxmem_picoset:.0%}', xy=(0, maxmem_picoset), xytext=(101, maxmem_picoset), color="grey")

plt.annotate(f'{fill100_nanoset}%', xy=(0, 1.24), xytext=(fill100_nanoset, 1.42), color="grey")
plt.annotate(f'{fill100_picoset}%', xy=(0, 1.24), xytext=(fill100_picoset, 1.42), color="grey")

plt.savefig(os.path.abspath(os.path.join(__file__, '..', 'sizegraph.svg')))
