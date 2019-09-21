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

plt.figure(figsize=[12, 8])
plt.xkcd(scale=0.5)
plt.axvline(76, linewidth=0.5, linestyle='--', color="grey")
plt.axvline(89.5, linewidth=0.5, linestyle='--', color="grey")
plt.axhline(1.1, linewidth=0.5, linestyle='--', color="grey")
plt.axhline(1.24, linewidth=0.5, linestyle='--', color="grey")
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
plt.annotate('124%', xy=(0, 1.24), xytext=(-7, 1.24), color="grey")
plt.annotate('110%', xy=(0, 1.24), xytext=(-6.8, 1.08), color="grey")

plt.annotate('76%', xy=(0, 1.24), xytext=(73, -0.0725), color="grey")
plt.annotate('90%', xy=(0, 1.24), xytext=(88, -0.0725), color="grey")



plt.savefig(os.path.abspath(os.path.join(__file__, '..', 'sizegraph.svg')))
