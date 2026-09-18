import json
import os

def write_dict_to_disk(data, filename="playlist.json"):
    """Writes a dictionary to disk in JSON format."""
    try:
        # Atomic write pattern: Write to temp file first to prevent corruption
        temp_filename = f"{filename}.tmp"
        with open(temp_filename, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=4)
            
        # Replace original file safely
        os.replace(temp_filename, filename)
    except Exception as e:
        print(f"[Error] Failed to write dictionary to disk: {e}")

def read_dict_from_disk(filename="playlist.json"):
    """Reads a dictionary from disk in JSON format."""
    try:
        if os.path.exists(filename):
            with open(filename, "r", encoding="utf-8") as f:
                return json.load(f)
        else:
            return {}
    except Exception as e:
        print(f"[Error] Failed to read dictionary from disk: {e}")
    return {}
