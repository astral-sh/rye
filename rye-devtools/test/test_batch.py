import rye_devtools


def test_batched():
    assert list(rye_devtools.batched("ABCDEFG", 3)) == ["ABC", "DEF", "G"]
