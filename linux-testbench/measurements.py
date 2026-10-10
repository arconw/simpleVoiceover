import math


def tone_amplitude(samples, rate, frequency):
    width = rate // 10
    powers = []
    for start in range(0, len(samples) - width + 1, width):
        window = samples[start : start + width]
        cosine = sum(
            value * math.cos(2 * math.pi * frequency * index / rate)
            for index, value in enumerate(window)
        )
        sine = sum(
            value * math.sin(2 * math.pi * frequency * index / rate)
            for index, value in enumerate(window)
        )
        powers.append((2 * math.hypot(cosine, sine) / width) ** 2)
    if not powers:
        raise ValueError("Recording is too short to verify its source tone")
    return math.sqrt(sum(powers) / len(powers))
