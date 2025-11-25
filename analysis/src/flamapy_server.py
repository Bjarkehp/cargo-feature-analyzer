import shlex
import sys

from flamapy.interfaces.python.flamapy_feature_model import FLAMAFeatureModel
from flamapy.core.exceptions import FlamaException

def main():
    model_path = None
    model = None
    try:
        while True:
            sys.stdout.flush()
            args = shlex.split(input())
            match args[0]:
                case "set_model":
                    model_path = args[1]
                    model = FLAMAFeatureModel(model_path)
                case "estimated_number_of_configurations":
                    if model == None:
                        print("Error: Model not assigned")
                        continue

                    result = model.estimated_number_of_configurations()
                    if result != None:
                        print(result)
                case "configurations_number":
                    if model == None:
                        print("Error: Model not assigned")
                        continue

                    result = model.configurations_number()
                    if result != None:
                        print(result)
                case "satisfiable_configuration":
                    if model == None:
                        print("Error: Model not assigned")
                        continue

                    configuration_path = args[1]
                    result = model.satisfiable_configuration(configuration_path)
                    if result != None:
                        print(result)
                case _:
                    print("Error: Command invalid")
    except FlamaException as e:
        print(f"Error with model {model_path}:")
        print(e)
    except EOFError:
        return

if __name__ == '__main__':
    main()