"""Bounded semantic comparison of preformatted fancy-grid table cells."""

from decimal import Decimal
import re


def numeric_identity(value: str) -> str:
	# Tuple arithmetic avoids Decimal context rounding of long authored values.
	sign, digits, exponent = Decimal(value).as_tuple()
	digits = list(digits)
	while digits and digits[-1] == 0:
		digits.pop()
		exponent += 1
	if not digits:
		return "0"
	return ("-" if sign else "") + "".join(str(digit) for digit in digits) + "e" + str(exponent)


def normalize(text: str) -> str:
	lines = text.splitlines()
	result = []
	index = 0
	while index < len(lines):
		line = lines[index].strip()
		tolerance = re.fullmatch(r"\(Note: Answer must be within \u00b1([+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?) of the correct value\)", line)
		if tolerance:
			lines[index] = "(Note: Answer must be within \u00b1" + numeric_identity(tolerance.group(1)) + " of the correct value)"
		if not (line.startswith("\u2552") and line.endswith("\u2555")):
			result.append(lines[index])
			index += 1
			continue
		end = index + 1
		while end < len(lines) and not lines[end].strip().startswith("\u2558"):
			end += 1
		block = [value.strip() for value in lines[index:end + 1]]
		width = line.count("\u2564") + 1
		valid = end < len(lines) and block[-1].endswith("\u255b")
		for value in block:
			if value.startswith("\u2502"):
				valid = valid and value.endswith("\u2502") and value.count("\u2502") == width + 1
			else:
				valid = valid and bool(re.fullmatch("[\u2500\u2550\u2552\u2564\u2555\u255e\u256a\u2561\u251c\u253c\u2524\u2558\u2567\u255b]+", value))
		if not valid:
			result.append(lines[index])
			index += 1
			continue
		for value in block:
			if value.startswith("\u2502"):
				cells = []
				for cell in value.split("\u2502")[1:-1]:
					cell = cell.strip()
					if re.fullmatch(r"[+-]?\d{1,3}(?:,\d{3})+(?:\.\d+)?", cell):
						cell = cell.replace(",", "")
					if re.fullmatch(r"[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?", cell):
						cell = numeric_identity(cell)
					cells.append(cell)
				result.append("\u2502" + "\u2502".join(cells) + "\u2502")
			else:
				result.append(re.sub("[\u2500\u2550]+", "\u2500", value))
		index = end + 1
	return "\n".join(result)


def selftest() -> None:
	left = "\u2552\u2550\u2564\u2550\u2555\n\u2502x\u2502 40\u2502\n\u2558\u2550\u2567\u2550\u255b"
	right = left.replace("40", "40.0").replace("\u2550", "\u2550\u2550")
	assert normalize(left) == normalize(right)
	assert normalize(left) != normalize(right.replace("40.0", "41.0"))
	assert normalize(left) != normalize(right.replace("x", "y"))
	assert normalize(left) != normalize(right.replace("x\u2502 40.0", "40.0\u2502 x"))
	assert normalize("Answer 40") != normalize("Answer 40.0")
	assert numeric_identity("123456789012345678901234567890") != numeric_identity("123456789012345678901234567891")
	assert normalize(left.replace("40", "4,400")) == normalize(left.replace("40", "4400"))
	assert normalize(left.replace("40", "4,40")) != normalize(left.replace("40", "440"))
	tolerance = "(Note: Answer must be within \u00b1500 of the correct value)"
	assert normalize(tolerance) == normalize(tolerance.replace("500", "500.0"))
	assert normalize(tolerance) != normalize(tolerance.replace("500", "501"))
