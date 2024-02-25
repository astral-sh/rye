from rye_devtools.find_downloads import batched


def test_batched():
    assert list(batched("ABCDEFG", 3)) == [tuple("ABC"), tuple("DEF"), tuple("G")]
