function lab-default-a { python3 -B /opt/linux-testbench/audio.py default a; }
function lab-default-b { python3 -B /opt/linux-testbench/audio.py default b; }
function lab-remove-a { python3 -B /opt/linux-testbench/audio.py remove a; }
function lab-remove-b { python3 -B /opt/linux-testbench/audio.py remove b; }
function lab-restore-a { python3 -B /opt/linux-testbench/audio.py restore a; }
function lab-restore-b { python3 -B /opt/linux-testbench/audio.py restore b; }
function lab-devices { python3 -B /opt/linux-testbench/audio.py devices; }
printf '%s\n' 'Linux test bench: two independent outputs and two microphones.' 'Fixtures: /fixtures. Exports and project files: /results.' 'Controls: lab-default-a, lab-default-b, lab-remove-a, lab-remove-b, lab-restore-a, lab-restore-b, lab-devices.'
