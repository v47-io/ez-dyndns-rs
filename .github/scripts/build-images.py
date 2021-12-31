#!/usr/bin/env python3

#  BSD 3-Clause License
#
#  Copyright (c) 2021, Alex Katlein
#  All rights reserved.
#
#  Redistribution and use in source and binary forms, with or without
#  modification, are permitted provided that the following conditions are met:
#
#  1. Redistributions of source code must retain the above copyright notice, this
#     list of conditions and the following disclaimer.
#
#  2. Redistributions in binary form must reproduce the above copyright notice,
#     this list of conditions and the following disclaimer in the documentation
#     and/or other materials provided with the distribution.
#
#  3. Neither the name of the copyright holder nor the names of its
#     contributors may be used to endorse or promote products derived from
#     this software without specific prior written permission.
#
#  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
#  AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
#  IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
#  DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
#  FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
#  DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
#  SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
#  CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
#  OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
#  OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
#


import re
from os import listdir
from os.path import abspath, join
from subprocess import run

from dockerfile import prepare_dockerfile
from version import version_number

EXEC_REGEX = re.compile(r'^dyndns-[a-z0-9-]+?$')

root_dir = abspath('./target/x86_64-unknown-linux-musl/release')

executables = filter(lambda name: EXEC_REGEX.match(name), listdir(root_dir))

for executable in executables:
    print(f"Building docker image for {executable}")

    image_name = f"v47io/ez-{executable}:r{version_number}"
    latest_name = f"v47io/ez-{executable}:latest"

    prepare_dockerfile(executable)

    print(f"Image name: {image_name}")

    run(['chmod', '+x', join(root_dir, executable)]).check_returncode()
    run(['docker', 'build', '-t', image_name, '.']).check_returncode()

    run(['docker', 'push', image_name]).check_returncode()

    run(['docker', 'tag', image_name, latest_name]).check_returncode()
    run(['docker', 'push', latest_name]).check_returncode()
