import json
import platform

print(
    json.dumps(
        {
            "python_implementation": platform.python_implementation(),
            "python_version": platform.python_version(),
        }
    )
)
