import os

from matplotlib import pyplot as plt
from matplotlib.ticker import PercentFormatter

S = 232
s_pico = 24
s_nano = 56

def with_set(x):
    return 100 * S / (100 * S)

def with_nanoset(x):
    return (x*(S + s_nano) + (100 - x)*s_nano) / (100 * S)

def with_picoset(x):
    return (x*(S + s_pico) + (100 - x)*s_pico) / (100 * S)

listx = list(range(101))

plt.plot(listx, list(map(with_set, listx)), label="with `set`")
plt.plot(listx, list(map(with_nanoset, listx)), label="with `nanoset.NanoSet`")
plt.plot(listx, list(map(with_picoset, listx)), label="with `nanoset.PicoSet`")
plt.legend([
    "with $set$",
    "with $nanoset.NanoSet$",
    "with $nanoset.PicoSet$",
])
plt.xlim([0, 100])
plt.ylim([0, 1.4])
plt.gca().yaxis.set_major_formatter(PercentFormatter(xmax=1))
plt.gca().xaxis.set_major_formatter(PercentFormatter(xmax=100))
plt.xlabel("Ratio of non-empty sets")
plt.ylabel("Ratio of memory used")
plt.axvline(89.5, color="grey")
plt.grid()
plt.savefig(os.path.abspath(os.path.join(__file__, '..', 'sizegraph.svg')))
